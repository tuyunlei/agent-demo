use std::sync::Arc;

use agent_domain::{
    AgentError, ChatMessage, FinishReason, LlmProvider, LlmRequest, MessageStore, ToolCall,
    ToolRuntime, ToolSpec,
};
use chrono::{DateTime, TimeZone, Utc};
use chrono_tz::Tz;

const CONTEXT_TIMEZONE: &str = "Asia/Shanghai";
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
        let tool_specs = self.tool_runtime.list_tools();
        let timezone = parse_timezone(CONTEXT_TIMEZONE);
        let system_prompt = build_system_prompt(&tool_specs, timezone);
        let mut messages = self.build_messages(history, &system_prompt, timezone);

        for _ in 0..MAX_TOOL_ROUNDS {
            let request = LlmRequest {
                messages: messages.clone(),
                model: None,
                temperature: None,
                max_tokens: None,
                tools: tool_specs.clone(),
            };

            let response = self.llm_provider.generate(request).await?;
            match response.finish_reason {
                FinishReason::Stop => {
                    self.message_store
                        .save_message(&session_id, "assistant", &response.content)
                        .await?;
                    return Ok(HandleMessageResult {
                        session_id,
                        reply: response.content,
                    });
                }
                FinishReason::ToolCalls => {
                    self.save_assistant_tool_call_message(&session_id, &response.tool_calls)
                        .await?;
                    self.append_assistant_tool_calls(&mut messages, response.tool_calls.clone());
                    self.execute_tool_calls(&session_id, &mut messages, response.tool_calls)
                        .await?;
                }
                FinishReason::Length => {
                    if response.content.is_empty() {
                        return Err(AgentError::InvalidInput(
                            "response truncated with empty content".to_string(),
                        ));
                    }

                    self.message_store
                        .save_message(&session_id, "assistant", &response.content)
                        .await?;
                    return Ok(HandleMessageResult {
                        session_id,
                        reply: response.content,
                    });
                }
            }
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

    fn build_messages(
        &self,
        history: Vec<agent_domain::StoredMessage>,
        system_prompt: &str,
        timezone: Tz,
    ) -> Vec<ChatMessage> {
        let mut messages = vec![ChatMessage {
            role: "system".to_string(),
            content: system_prompt.to_string(),
            tool_calls: None,
            tool_call_id: None,
        }];

        messages.extend(history.into_iter().map(|item| ChatMessage {
            role: item.role.clone(),
            content: format_message_content(&item.role, &item.content, item.created_at, timezone),
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
        let message = ChatMessage {
            role: "assistant".to_string(),
            content: String::new(),
            tool_calls: Some(tool_calls.to_vec()),
            tool_call_id: None,
        };
        self.message_store
            .save_message_ext(session_id, &message)
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
            let message = ChatMessage {
                role: "tool".to_string(),
                content: result.content,
                tool_calls: None,
                tool_call_id: Some(call_id),
            };
            self.message_store
                .save_message_ext(session_id, &message)
                .await?;
            messages.push(message);
        }
        Ok(())
    }
}

fn parse_timezone(name: &str) -> Tz {
    name.parse().unwrap_or(chrono_tz::UTC)
}

fn format_message_content(role: &str, content: &str, created_at: i64, timezone: Tz) -> String {
    if role != "user" && role != "assistant" {
        return content.to_string();
    }

    match Utc.timestamp_opt(created_at, 0).single() {
        Some(timestamp) => format!("[{}] {content}", format_timestamp(timestamp, timezone)),
        None => content.to_string(),
    }
}

fn format_timestamp(timestamp: DateTime<Utc>, timezone: Tz) -> String {
    timestamp
        .with_timezone(&timezone)
        .format("%Y-%m-%d %H:%M")
        .to_string()
}

fn build_system_prompt(tool_specs: &[ToolSpec], timezone: Tz) -> String {
    let now = format_timestamp(Utc::now(), timezone);
    let tools = if tool_specs.is_empty() {
        "- (none)".to_string()
    } else {
        tool_specs
            .iter()
            .map(|tool| format!("- {}: {}", tool.name, tool.description))
            .collect::<Vec<_>>()
            .join("\n")
    };

    format!(
        "You are a helpful AI assistant.\n\nCurrent time: {now} ({timezone})\n\nYou have access to the following tools:\n{tools}\n\nWhen the user asks about current events, time, or facts you're unsure about, use the appropriate tool.\nBe concise and helpful. Respond in the same language the user uses."
    )
}

#[cfg(test)]
#[path = "runtime_tests.rs"]
mod tests;
