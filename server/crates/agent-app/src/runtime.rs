use std::sync::Arc;

use agent_domain::{
    AgentError, ChatMessage, FinishReason, LlmProvider, LlmRequest, MessageStore, ToolCall,
    ToolRuntime,
};

const SYSTEM_PROMPT: &str = "You are a helpful assistant.";
const MAX_TOOL_ROUNDS: usize = 10;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HandleMessageResult {
    pub session_id: String,
    pub reply: String,
}

pub struct AgentRuntime {
    llm_provider: Arc<dyn LlmProvider + Send + Sync>,
    message_store: Arc<dyn MessageStore + Send + Sync>,
    tool_runtime: Arc<dyn ToolRuntime + Send + Sync>,
}

impl AgentRuntime {
    pub fn new(
        llm_provider: Arc<dyn LlmProvider + Send + Sync>,
        message_store: Arc<dyn MessageStore + Send + Sync>,
        tool_runtime: Arc<dyn ToolRuntime + Send + Sync>,
    ) -> Self {
        Self {
            llm_provider,
            message_store,
            tool_runtime,
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

        let session_id = self.resolve_session_id(user_id, session_id).await?;
        self.message_store
            .save_message(&session_id, "user", user_message)
            .await?;

        let history = self
            .message_store
            .get_session_messages(&session_id, 50)
            .await?;
        let mut messages = self.build_messages(history);
        let tool_specs = self.tool_runtime.list_tools();

        for _ in 0..MAX_TOOL_ROUNDS {
            let request = LlmRequest {
                messages: messages.clone(),
                model: None,
                temperature: None,
                max_tokens: None,
                tools: tool_specs.clone(),
            };

            let response = self.llm_provider.generate(request).await?;
            if response.finish_reason == FinishReason::Stop {
                self.message_store
                    .save_message(&session_id, "assistant", &response.content)
                    .await?;
                return Ok(HandleMessageResult {
                    session_id,
                    reply: response.content,
                });
            }

            self.save_assistant_tool_call_message(&session_id, &response.tool_calls)
                .await?;
            self.append_assistant_tool_calls(&mut messages, response.tool_calls.clone());
            self.execute_tool_calls(&session_id, &mut messages, response.tool_calls)
                .await?;
        }

        Err(AgentError::InvalidInput(
            "max tool rounds exceeded".to_string(),
        ))
    }

    async fn resolve_session_id(
        &self,
        user_id: &str,
        session_id: Option<&str>,
    ) -> Result<String, AgentError> {
        match session_id {
            Some(id) if !id.trim().is_empty() => Ok(id.to_string()),
            _ => Ok(self
                .message_store
                .get_or_create_default_session(user_id)
                .await?),
        }
    }

    fn build_messages(&self, history: Vec<agent_domain::StoredMessage>) -> Vec<ChatMessage> {
        let mut messages = vec![ChatMessage {
            role: "system".to_string(),
            content: SYSTEM_PROMPT.to_string(),
            tool_calls: None,
            tool_call_id: None,
        }];

        messages.extend(history.into_iter().map(|item| ChatMessage {
            role: item.role,
            content: item.content,
            tool_calls: None,
            tool_call_id: None,
        }));

        messages
    }

    async fn save_assistant_tool_call_message(
        &self,
        session_id: &str,
        tool_calls: &[ToolCall],
    ) -> Result<(), AgentError> {
        let encoded = serde_json::to_string(tool_calls)
            .map_err(|e| AgentError::InvalidInput(format!("failed to encode tool_calls: {e}")))?;
        self.message_store
            .save_message(session_id, "assistant", &encoded)
            .await?;
        Ok(())
    }

    fn append_assistant_tool_calls(
        &self,
        messages: &mut Vec<ChatMessage>,
        tool_calls: Vec<ToolCall>,
    ) {
        messages.push(ChatMessage {
            role: "assistant".to_string(),
            content: String::new(),
            tool_calls: Some(tool_calls),
            tool_call_id: None,
        });
    }

    async fn execute_tool_calls(
        &self,
        session_id: &str,
        messages: &mut Vec<ChatMessage>,
        tool_calls: Vec<ToolCall>,
    ) -> Result<(), AgentError> {
        for tool_call in tool_calls {
            let result = self
                .tool_runtime
                .execute(&tool_call.name, &tool_call.arguments)
                .await?;
            let call_id = if result.call_id.is_empty() {
                tool_call.call_id
            } else {
                result.call_id
            };
            self.message_store
                .save_message(session_id, "tool", &result.content)
                .await?;
            messages.push(ChatMessage {
                role: "tool".to_string(),
                content: result.content,
                tool_calls: None,
                tool_call_id: Some(call_id),
            });
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "runtime_tests.rs"]
mod tests;
