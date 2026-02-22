use std::sync::Arc;

use agent_domain::{AgentError, ChatMessage, LlmProvider, LlmRequest, MessageStore};

const SYSTEM_PROMPT: &str = "You are a helpful assistant.";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HandleMessageResult {
    pub session_id: String,
    pub reply: String,
}

pub struct AgentRuntime {
    llm_provider: Arc<dyn LlmProvider + Send + Sync>,
    message_store: Arc<dyn MessageStore + Send + Sync>,
}

impl AgentRuntime {
    pub fn new(
        llm_provider: Arc<dyn LlmProvider + Send + Sync>,
        message_store: Arc<dyn MessageStore + Send + Sync>,
    ) -> Self {
        Self {
            llm_provider,
            message_store,
        }
    }

    pub async fn handle_message(
        &self,
        user_id: &str,
        session_id: Option<&str>,
        user_message: &str,
    ) -> Result<HandleMessageResult, AgentError> {
        if user_message.trim().is_empty() {
            return Err(AgentError::InvalidInput(
                "message content cannot be empty".into(),
            ));
        }

        let session_id = match session_id {
            Some(id) if !id.trim().is_empty() => id.to_string(),
            _ => {
                self.message_store
                    .get_or_create_default_session(user_id)
                    .await?
            }
        };

        self.message_store
            .save_message(&session_id, "user", user_message)
            .await?;

        let history = self
            .message_store
            .get_session_messages(&session_id, 50)
            .await?;

        let mut messages = vec![ChatMessage {
            role: "system".to_string(),
            content: SYSTEM_PROMPT.to_string(),
        }];
        messages.extend(history.into_iter().map(|item| ChatMessage {
            role: item.role,
            content: item.content,
        }));

        let request = LlmRequest {
            messages,
            model: None,
            temperature: None,
            max_tokens: None,
        };

        let response = self.llm_provider.generate(request).await?;

        self.message_store
            .save_message(&session_id, "assistant", &response.content)
            .await?;

        Ok(HandleMessageResult {
            session_id,
            reply: response.content,
        })
    }
}

#[cfg(test)]
#[path = "runtime_tests.rs"]
mod tests;
