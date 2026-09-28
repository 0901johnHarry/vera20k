use std::io::{Read, Take};
use std::time::Duration;

use serde_json::Value;

use super::protocol::{
    ExternalAiDecision, ExternalAiError, build_chat_request, parse_chat_completion,
};

const MAX_RESPONSE_BYTES: u64 = 65_536;

/// Blocking transport. It is constructed and called only by the app worker.
/// The credential is intentionally not exposed through `Debug` or errors.
pub(super) struct ExternalAiClient {
    agent: ureq::Agent,
    endpoint: String,
    model: String,
    api_key: Option<String>,
}

impl ExternalAiClient {
    pub(super) fn new(
        endpoint: &str,
        model: &str,
        timeout: Duration,
        api_key: Option<&str>,
    ) -> Self {
        let tls = ureq::tls::TlsConfig::builder()
            .root_certs(ureq::tls::RootCerts::PlatformVerifier)
            .build();
        let agent = ureq::Agent::config_builder()
            .timeout_global(Some(timeout))
            .tls_config(tls)
            .build()
            .new_agent();

        Self {
            agent,
            endpoint: endpoint.to_string(),
            model: model.to_string(),
            api_key: api_key
                .filter(|key| !key.trim().is_empty())
                .map(str::to_string),
        }
    }

    pub(super) fn request(
        &self,
        observation: &Value,
        max_actions: usize,
    ) -> Result<ExternalAiDecision, ExternalAiError> {
        let body = build_chat_request(&self.model, observation);
        let request = self
            .agent
            .post(&self.endpoint)
            .header("Content-Type", "application/json");
        let response = match self.api_key.as_deref() {
            Some(api_key) => request.header("Authorization", &format!("Bearer {api_key}")),
            None => request,
        }
        .send_json(&body);

        let mut response = match response {
            Ok(response) => response,
            Err(ureq::Error::StatusCode(status)) => {
                return Err(ExternalAiError::HttpStatus(status));
            }
            Err(_) => return Err(ExternalAiError::Transport),
        };

        let mut limited: Take<_> = response.body_mut().as_reader().take(MAX_RESPONSE_BYTES + 1);
        let mut response_body = Vec::with_capacity(2048);
        limited
            .read_to_end(&mut response_body)
            .map_err(|_| ExternalAiError::Transport)?;
        if response_body.len() as u64 > MAX_RESPONSE_BYTES {
            return Err(ExternalAiError::ResponseTooLarge);
        }
        parse_chat_completion(&response_body, max_actions)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};
    use std::sync::mpsc::{self, Receiver};
    use std::thread::{self, JoinHandle};
    use std::time::{Duration, Instant};

    use serde_json::json;

    use crate::app::match_runtime::external_ai::worker::{ExternalAiWorker, RequestTag};
    use crate::sim::intern::InternedId;
    use crate::util::config::ExternalAiConfig;

    use super::ExternalAiClient;

    struct MockServer {
        endpoint: String,
        request: Receiver<Vec<u8>>,
        thread: JoinHandle<()>,
    }

    struct ConcurrentMockServer {
        endpoint: String,
        both_arrived_before_response: Receiver<bool>,
        thread: JoinHandle<()>,
    }

    impl ConcurrentMockServer {
        fn new() -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").expect("bind concurrent mock API");
            let address = listener.local_addr().unwrap();
            listener.set_nonblocking(true).unwrap();
            let (arrivals_tx, both_arrived_before_response) = mpsc::channel();
            let thread = thread::spawn(move || {
                let mut streams = Vec::with_capacity(2);
                let accept_deadline = Instant::now() + Duration::from_secs(3);
                while streams.is_empty() && Instant::now() < accept_deadline {
                    match listener.accept() {
                        Ok((mut stream, _)) => {
                            stream
                                .set_read_timeout(Some(Duration::from_secs(3)))
                                .unwrap();
                            let _ = read_http_request(&mut stream);
                            streams.push(stream);
                        }
                        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                            thread::sleep(Duration::from_millis(5));
                        }
                        Err(error) => panic!("accept first worker request: {error}"),
                    }
                }

                let concurrency_deadline = Instant::now() + Duration::from_secs(1);
                while streams.len() < 2 && Instant::now() < concurrency_deadline {
                    match listener.accept() {
                        Ok((mut stream, _)) => {
                            stream
                                .set_read_timeout(Some(Duration::from_secs(3)))
                                .unwrap();
                            let _ = read_http_request(&mut stream);
                            streams.push(stream);
                        }
                        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                            thread::sleep(Duration::from_millis(5));
                        }
                        Err(error) => panic!("accept concurrent worker request: {error}"),
                    }
                }
                arrivals_tx.send(streams.len() == 2).unwrap();

                for stream in &mut streams {
                    write_valid_response(stream);
                }

                if streams.len() == 1 {
                    let late_deadline = Instant::now() + Duration::from_secs(3);
                    while Instant::now() < late_deadline {
                        match listener.accept() {
                            Ok((mut stream, _)) => {
                                stream
                                    .set_read_timeout(Some(Duration::from_secs(3)))
                                    .unwrap();
                                let _ = read_http_request(&mut stream);
                                write_valid_response(&mut stream);
                                break;
                            }
                            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                                thread::sleep(Duration::from_millis(5));
                            }
                            Err(error) => panic!("accept queued worker request: {error}"),
                        }
                    }
                }
            });
            Self {
                endpoint: format!("http://{address}/v1/chat/completions"),
                both_arrived_before_response,
                thread,
            }
        }

        fn join(self) -> bool {
            let both_arrived = self
                .both_arrived_before_response
                .recv_timeout(Duration::from_secs(4))
                .expect("mock server reports request concurrency");
            self.thread.join().expect("concurrent mock API thread");
            both_arrived
        }
    }

    impl MockServer {
        fn new(status: u16, reason: &'static str, body: Vec<u8>, delay: Duration) -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").expect("bind mock API endpoint");
            let address = listener.local_addr().unwrap();
            let (request_tx, request) = mpsc::channel();
            let thread = thread::spawn(move || {
                let (mut stream, _) = listener.accept().expect("accept API request");
                stream
                    .set_read_timeout(Some(Duration::from_secs(3)))
                    .unwrap();
                let request = read_http_request(&mut stream);
                request_tx.send(request).unwrap();
                thread::sleep(delay);
                let header = format!(
                    "HTTP/1.1 {status} {reason}\r\nContent-Length: {}\r\nConnection: close\r\nContent-Type: application/json\r\n\r\n",
                    body.len()
                );
                let _ = stream.write_all(header.as_bytes());
                let _ = stream.write_all(&body);
                let _ = stream.flush();
            });
            Self {
                endpoint: format!("http://{address}/v1/chat/completions"),
                request,
                thread,
            }
        }

        fn join(self) -> Vec<u8> {
            let request = self.request.recv_timeout(Duration::from_secs(3)).unwrap();
            self.thread.join().expect("mock endpoint thread");
            request
        }
    }

    fn read_http_request(stream: &mut TcpStream) -> Vec<u8> {
        let mut request = Vec::new();
        let mut buffer = [0_u8; 1024];
        let header_end = loop {
            let count = stream.read(&mut buffer).expect("read request header");
            assert_ne!(count, 0, "request closed before its headers");
            request.extend_from_slice(&buffer[..count]);
            if let Some(position) = request.windows(4).position(|bytes| bytes == b"\r\n\r\n") {
                break position + 4;
            }
        };
        let headers = String::from_utf8_lossy(&request[..header_end]);
        let content_length = headers
            .lines()
            .find_map(|line| {
                let (name, value) = line.split_once(':')?;
                name.eq_ignore_ascii_case("content-length")
                    .then(|| value.trim().parse::<usize>().unwrap())
            })
            .expect("request content length");
        while request.len() < header_end + content_length {
            let count = stream.read(&mut buffer).expect("read request body");
            assert_ne!(count, 0, "request closed before its body");
            request.extend_from_slice(&buffer[..count]);
        }
        request
    }

    fn valid_response() -> Vec<u8> {
        br#"{"choices":[{"message":{"role":"assistant","content":"{\"actions\":[]}"}}]}"#.to_vec()
    }

    fn write_valid_response(stream: &mut TcpStream) {
        let body = valid_response();
        let header = format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\nContent-Type: application/json\r\n\r\n",
            body.len()
        );
        let _ = stream.write_all(header.as_bytes());
        let _ = stream.write_all(&body);
        let _ = stream.flush();
    }

    #[test]
    fn separate_ai_houses_do_not_wait_for_each_others_http_responses() {
        let server = ConcurrentMockServer::new();
        let config = ExternalAiConfig {
            enabled: true,
            endpoint: server.endpoint.clone(),
            model: "fixture-model".into(),
            request_timeout_secs: 3,
            ..ExternalAiConfig::default()
        };
        let owners = [InternedId::from_index(1), InternedId::from_index(2)];
        let worker = ExternalAiWorker::start(&config, &owners).expect("start external AI worker");
        for owner in owners {
            worker
                .try_submit(
                    RequestTag {
                        match_generation: 1,
                        owner,
                        source_frame: 225,
                    },
                    json!({"owner": owner.index()}),
                    16,
                )
                .expect("enqueue request");
        }

        let both_arrived_before_response = server.join();
        let mut results = Vec::new();
        let deadline = Instant::now() + Duration::from_secs(2);
        while results.len() < 2 && Instant::now() < deadline {
            match worker.try_receive().expect("worker result channel") {
                Some(result) => results.push(result),
                None => thread::sleep(Duration::from_millis(5)),
            }
        }

        assert!(
            both_arrived_before_response,
            "each house request should reach the provider before either response is released"
        );
        assert!(results.iter().all(|result| result.result.is_ok()));
        assert_eq!(
            results
                .iter()
                .map(|result| result.tag.owner)
                .collect::<HashSet<_>>(),
            owners.into_iter().collect()
        );
    }

    #[test]
    fn configured_bearer_key_is_sent_and_omitted_when_absent() {
        for (api_key, authorization) in [
            (
                Some("local-test-token"),
                Some("authorization: bearer local-test-token"),
            ),
            (None, None),
        ] {
            let server = MockServer::new(200, "OK", valid_response(), Duration::ZERO);
            let client = ExternalAiClient::new(
                &server.endpoint,
                "fixture-model",
                Duration::from_secs(2),
                api_key,
            );
            client
                .request(&json!({"owner":"alpha"}), 16)
                .expect("successful mock response");
            let request_bytes = server.join();
            let request = String::from_utf8_lossy(&request_bytes).to_ascii_lowercase();
            match authorization {
                Some(expected) => assert!(request.contains(expected)),
                None => assert!(!request.contains("authorization:")),
            }
            let body_start = request_bytes
                .windows(4)
                .position(|bytes| bytes == b"\r\n\r\n")
                .expect("HTTP request headers")
                + 4;
            let body: serde_json::Value =
                serde_json::from_slice(&request_bytes[body_start..]).expect("request JSON");
            assert_eq!(body["model"], "fixture-model");
        }
    }

    #[test]
    fn provider_error_body_is_not_exposed_in_the_client_error() {
        let server = MockServer::new(
            429,
            "Too Many Requests",
            br#"{"error":"local-test-token must not escape"}"#.to_vec(),
            Duration::ZERO,
        );
        let client = ExternalAiClient::new(
            &server.endpoint,
            "fixture-model",
            Duration::from_secs(2),
            Some("local-test-token"),
        );
        let error = client.request(&json!({"owner":"alpha"}), 16).unwrap_err();
        let _ = server.join();
        assert_eq!(error.to_string(), "provider returned HTTP status 429");
        assert!(!error.to_string().contains("local-test-token"));
    }

    #[test]
    fn configured_timeout_bounds_a_slow_local_endpoint() {
        let server = MockServer::new(200, "OK", valid_response(), Duration::from_millis(1300));
        let client = ExternalAiClient::new(
            &server.endpoint,
            "fixture-model",
            Duration::from_secs(1),
            None,
        );
        let started = std::time::Instant::now();
        assert!(client.request(&json!({"owner":"alpha"}), 16).is_err());
        assert!(started.elapsed() < Duration::from_secs(2));
        let _ = server.join();
    }

    #[test]
    fn streamed_response_body_is_capped_before_parsing() {
        let server = MockServer::new(200, "OK", vec![b'x'; 65_537], Duration::ZERO);
        let client = ExternalAiClient::new(
            &server.endpoint,
            "fixture-model",
            Duration::from_secs(2),
            None,
        );
        assert_eq!(
            client
                .request(&json!({"owner":"alpha"}), 16)
                .unwrap_err()
                .to_string(),
            "provider response exceeded the 65536 byte limit"
        );
        let _ = server.join();
    }
}
