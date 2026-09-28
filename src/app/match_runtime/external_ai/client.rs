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
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};
    use std::sync::mpsc::{self, Receiver};
    use std::thread::{self, JoinHandle};
    use std::time::Duration;

    use serde_json::json;

    use super::ExternalAiClient;

    struct MockServer {
        endpoint: String,
        request: Receiver<Vec<u8>>,
        thread: JoinHandle<()>,
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
