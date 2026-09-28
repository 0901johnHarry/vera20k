use serde::{Deserialize, Serialize};

const MAX_RESPONSE_BYTES: usize = 65_536;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum ExternalAiAction {
    QueueProduction {
        type_id: String,
    },
    Move {
        entity_id: u64,
        target_rx: u16,
        target_ry: u16,
        #[serde(default)]
        queue: bool,
    },
    AttackMove {
        entity_id: u64,
        target_rx: u16,
        target_ry: u16,
        #[serde(default)]
        queue: bool,
    },
    Guard {
        entity_id: u64,
        #[serde(default)]
        target_id: Option<u64>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ExternalAiDecision {
    pub(super) actions: Vec<ExternalAiAction>,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub(super) enum ExternalAiError {
    #[error("external AI configuration is invalid")]
    InvalidConfiguration,
    #[error("external AI worker could not be started")]
    WorkerStart,
    #[error("external AI worker is shutting down")]
    WorkerStopped,
    #[error("external AI worker queue is full")]
    WorkerQueueFull,
    #[error("provider transport failed or timed out")]
    Transport,
    #[error("provider returned HTTP status {0}")]
    HttpStatus(u16),
    #[error("provider response exceeded the 65536 byte limit")]
    ResponseTooLarge,
    #[error("provider response did not match the supported JSON schema")]
    InvalidResponse,
    #[error("provider response exceeded the configured action limit")]
    TooManyActions,
}

#[derive(Serialize)]
pub(super) struct ChatCompletionRequest {
    model: String,
    messages: [ChatMessage; 2],
}

#[derive(Serialize)]
struct ChatMessage {
    role: &'static str,
    content: String,
}

const SYSTEM_PROMPT: &str = "You control one computer house in a real-time strategy game. Use only facts in the supplied observation. Return only a JSON object with an actions array. Each action must be one of: {\"type\":\"queue_production\",\"type_id\":string}; {\"type\":\"move\",\"entity_id\":integer,\"target_rx\":integer,\"target_ry\":integer,\"queue\":boolean}; {\"type\":\"attack_move\",\"entity_id\":integer,\"target_rx\":integer,\"target_ry\":integer,\"queue\":boolean}; or {\"type\":\"guard\",\"entity_id\":integer,\"target_id\":integer|null}. Do not include prose, tools, or any other action.";

pub(super) fn build_chat_request(
    model: &str,
    observation: &serde_json::Value,
) -> ChatCompletionRequest {
    ChatCompletionRequest {
        model: model.to_string(),
        messages: [
            ChatMessage {
                role: "system",
                content: SYSTEM_PROMPT.to_string(),
            },
            ChatMessage {
                role: "user",
                content: observation.to_string(),
            },
        ],
    }
}

#[derive(Deserialize)]
struct ChatCompletionResponse {
    choices: Vec<ChatCompletionChoice>,
}

#[derive(Deserialize)]
struct ChatCompletionChoice {
    message: ChatCompletionMessage,
}

#[derive(Deserialize)]
struct ChatCompletionMessage {
    content: Option<String>,
}

pub(super) fn parse_chat_completion(
    body: &[u8],
    max_actions: usize,
) -> Result<ExternalAiDecision, ExternalAiError> {
    if body.len() > MAX_RESPONSE_BYTES {
        return Err(ExternalAiError::ResponseTooLarge);
    }
    let response: ChatCompletionResponse =
        serde_json::from_slice(body).map_err(|_| ExternalAiError::InvalidResponse)?;
    let content = response
        .choices
        .first()
        .and_then(|choice| choice.message.content.as_deref())
        .ok_or(ExternalAiError::InvalidResponse)?;
    if content.len() > MAX_RESPONSE_BYTES {
        return Err(ExternalAiError::ResponseTooLarge);
    }
    let decision: ExternalAiDecision =
        serde_json::from_str(content).map_err(|_| ExternalAiError::InvalidResponse)?;
    if decision.actions.len() > max_actions {
        return Err(ExternalAiError::TooManyActions);
    }
    Ok(decision)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{ExternalAiAction, ExternalAiError, build_chat_request, parse_chat_completion};

    #[test]
    fn chat_request_contains_only_model_prompt_and_the_supplied_observation() {
        let request = build_chat_request(
            "fixture-model",
            &json!({"owner": "alpha", "visible_enemies": []}),
        );
        let value = serde_json::to_value(request).expect("serialize chat request");

        assert_eq!(value["model"], "fixture-model");
        assert_eq!(value["messages"][0]["role"], "system");
        assert_eq!(value["messages"][1]["role"], "user");
        assert_eq!(
            value["messages"][1]["content"],
            r#"{"owner":"alpha","visible_enemies":[]}"#
        );
        assert!(
            value["messages"][0]["content"]
                .as_str()
                .expect("system prompt")
                .contains("attack_move")
        );
    }

    #[test]
    fn chat_completion_content_decodes_only_the_supported_action_schema() {
        let body = br#"{"choices":[{"message":{"role":"assistant","content":"{\"actions\":[{\"type\":\"queue_production\",\"type_id\":\"GI\"},{\"type\":\"move\",\"entity_id\":42,\"target_rx\":7,\"target_ry\":9}]}"}}]}"#;

        let decision = parse_chat_completion(body, 2).expect("valid completion");
        assert_eq!(
            decision.actions,
            vec![
                ExternalAiAction::QueueProduction {
                    type_id: "GI".to_string()
                },
                ExternalAiAction::Move {
                    entity_id: 42,
                    target_rx: 7,
                    target_ry: 9,
                    queue: false,
                },
            ]
        );
    }

    #[test]
    fn malformed_or_unknown_action_content_is_rejected() {
        let malformed = br#"{"choices":[{"message":{"content":"not json"}}]}"#;
        assert_eq!(
            parse_chat_completion(malformed, 16).unwrap_err(),
            ExternalAiError::InvalidResponse
        );

        let unknown_action = br#"{"choices":[{"message":{"content":"{\"actions\":[{\"type\":\"run_shell\",\"command\":\"x\"}]}"}}]}"#;
        assert_eq!(
            parse_chat_completion(unknown_action, 16).unwrap_err(),
            ExternalAiError::InvalidResponse
        );
    }

    #[test]
    fn completion_without_actions_or_over_action_limit_is_rejected() {
        let missing_actions = br#"{"choices":[{"message":{"content":"{}"}}]}"#;
        assert_eq!(
            parse_chat_completion(missing_actions, 16).unwrap_err(),
            ExternalAiError::InvalidResponse
        );

        let too_many = br#"{"choices":[{"message":{"content":"{\"actions\":[{\"type\":\"queue_production\",\"type_id\":\"GI\"},{\"type\":\"queue_production\",\"type_id\":\"HTNK\"}]}"}}]}"#;
        assert_eq!(
            parse_chat_completion(too_many, 1).unwrap_err(),
            ExternalAiError::TooManyActions
        );
    }

    #[test]
    fn oversized_completion_is_rejected_before_json_parsing() {
        let body = vec![b'x'; 65_537];
        assert_eq!(
            parse_chat_completion(&body, 16).unwrap_err(),
            ExternalAiError::ResponseTooLarge
        );
    }
}
