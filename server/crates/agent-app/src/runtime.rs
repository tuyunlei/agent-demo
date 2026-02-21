use std::sync::Arc;

use agent_domain::{AgentError, ChatMessage, LlmProvider, LlmRequest};

const SYSTEM_PROMPT: &str = "You are a helpful assistant.";

pub struct AgentRuntime {
    llm_provider: Arc<dyn LlmProvider + Send + Sync>,
}

impl AgentRuntime {
    pub fn new(llm_provider: Arc<dyn LlmProvider + Send + Sync>) -> Self {
        Self { llm_provider }
    }

    pub async fn handle_message(&self, user_message: &str) -> Result<String, AgentError> {
        if user_message.trim().is_empty() {
            return Err(AgentError::InvalidInput(
                "message content cannot be empty".into(),
            ));
        }

        let request = LlmRequest {
            messages: vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: SYSTEM_PROMPT.to_string(),
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: user_message.to_string(),
                },
            ],
            model: None,
            temperature: None,
            max_tokens: None,
        };

        let response = self
            .llm_provider
            .generate(request)
            .await
            .map_err(AgentError::from)?;

        Ok(response.content)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use agent_domain::{LlmError, LlmResponse, LlmUsage};

    use super::*;

    struct MockLlmProvider {
        captured: Arc<Mutex<Vec<LlmRequest>>>,
        response: Result<LlmResponse, LlmError>,
    }

    #[async_trait::async_trait]
    impl LlmProvider for MockLlmProvider {
        async fn generate(&self, request: LlmRequest) -> Result<LlmResponse, LlmError> {
            self.captured.lock().expect("lock captured").push(request);
            self.response.clone()
        }
    }

    #[tokio::test]
    async fn handle_message_builds_minimal_llm_request() {
        let captured = Arc::new(Mutex::new(Vec::new()));
        let runtime = AgentRuntime::new(Arc::new(MockLlmProvider {
            captured: captured.clone(),
            response: Ok(LlmResponse {
                content: "hello from ai".to_string(),
                model: "mock-model".to_string(),
                usage: Some(LlmUsage {
                    input_tokens: 1,
                    output_tokens: 1,
                    total_tokens: 2,
                }),
            }),
        }));

        let reply = runtime.handle_message("hi").await.expect("reply");

        assert_eq!(reply, "hello from ai");

        let requests = captured.lock().expect("lock captured");
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].messages.len(), 2);
        assert_eq!(requests[0].messages[0].role, "system");
        assert_eq!(requests[0].messages[0].content, SYSTEM_PROMPT);
        assert_eq!(requests[0].messages[1].role, "user");
        assert_eq!(requests[0].messages[1].content, "hi");
    }

    #[tokio::test]
    async fn handle_message_empty_input_returns_error() {
        let runtime = AgentRuntime::new(Arc::new(MockLlmProvider {
            captured: Arc::new(Mutex::new(Vec::new())),
            response: Err(LlmError::Timeout),
        }));

        let result = runtime.handle_message("").await;

        assert_eq!(
            result,
            Err(AgentError::InvalidInput(
                "message content cannot be empty".to_string()
            ))
        );
    }

    #[tokio::test]
    async fn handle_message_whitespace_only_returns_error() {
        let runtime = AgentRuntime::new(Arc::new(MockLlmProvider {
            captured: Arc::new(Mutex::new(Vec::new())),
            response: Err(LlmError::Timeout),
        }));

        let result = runtime.handle_message("   \n\t").await;

        assert_eq!(
            result,
            Err(AgentError::InvalidInput(
                "message content cannot be empty".to_string()
            ))
        );
    }

    #[tokio::test]
    async fn handle_message_llm_error_propagates() {
        let runtime = AgentRuntime::new(Arc::new(MockLlmProvider {
            captured: Arc::new(Mutex::new(Vec::new())),
            response: Err(LlmError::RateLimited),
        }));

        let result = runtime.handle_message("hello").await;

        assert_eq!(result, Err(AgentError::Llm(LlmError::RateLimited)));
    }
}
