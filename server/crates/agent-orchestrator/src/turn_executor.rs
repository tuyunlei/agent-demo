use std::sync::Arc;

use agent_domain::{ChatMessage, MessageStore, StoreError};
use agent_llm::LlmProvider;
use agent_llm::types::{
    FinishReason, LlmRequest, LlmRequestConfig, LlmRequestMetadata, LlmResponse,
};
use agent_memory::CompactionService;
use agent_tools::ToolRuntime;

use crate::turn_compat::{
    build_system_prompt, chat_message_to_model_message, domain_tool_spec_to_llm_spec,
    format_message_content, llm_tool_call_to_tool_input, parse_timezone, to_domain_call,
    tool_error_json, tool_output_to_chat_message, tool_specs_to_domain_specs,
};
use crate::turn_types::{TurnError, TurnExecutorConfig, TurnFinishReason, TurnInput, TurnOutput};

pub struct TurnExecutor {
    llm: Arc<dyn LlmProvider>,
    tools: Arc<dyn ToolRuntime>,
    message_store: Arc<dyn MessageStore>,
    compaction: Arc<dyn CompactionService>,
    config: TurnExecutorConfig,
}

impl TurnExecutor {
    pub fn new(
        llm: Arc<dyn LlmProvider>,
        tools: Arc<dyn ToolRuntime>,
        message_store: Arc<dyn MessageStore>,
        compaction: Arc<dyn CompactionService>,
        config: TurnExecutorConfig,
    ) -> Self {
        Self {
            llm,
            tools,
            message_store,
            compaction,
            config,
        }
    }

    pub async fn run_turn(&self, input: TurnInput) -> Result<TurnOutput, TurnError> {
        if input.user_message.trim().is_empty() {
            return Err(TurnError::InvalidInput(
                "message content cannot be empty".to_string(),
            ));
        }

        let session_id = self
            .resolve_session_id(&input.user_id, input.session_id.as_deref())
            .await?;
        self.message_store
            .save_message(&session_id, "user", &input.user_message)
            .await
            .map_err(store_err)?;

        let history = self
            .message_store
            .get_session_messages(&session_id, 50)
            .await
            .map_err(store_err)?;

        let timezone = parse_timezone(&self.config.timezone);
        let tool_specs = self.tools.list_specs();
        let domain_specs = tool_specs_to_domain_specs(&tool_specs);
        let system_prompt = build_system_prompt(&domain_specs, timezone);

        let mut messages = vec![ChatMessage {
            role: "system".to_string(),
            content: system_prompt,
            tool_calls: None,
            tool_call_id: None,
        }];
        messages.extend(history.into_iter().map(|item| ChatMessage {
            role: item.role.clone(),
            content: format_message_content(&item.role, &item.content, item.created_at, timezone),
            tool_calls: None,
            tool_call_id: None,
        }));

        let mut iteration: u8 = 0;
        loop {
            let request = LlmRequest {
                messages: messages
                    .iter()
                    .map(chat_message_to_model_message)
                    .collect::<Vec<_>>(),
                tool_specs: domain_specs
                    .iter()
                    .map(domain_tool_spec_to_llm_spec)
                    .collect::<Vec<_>>(),
                builtin_tools: vec![],
                previous_response_id: None,
                config: LlmRequestConfig {
                    model: "mock".to_string(),
                    temperature: None,
                    top_p: None,
                    max_tokens: None,
                    timeout: None,
                    json_mode: false,
                },
                metadata: LlmRequestMetadata {
                    session_id: Some(session_id.clone()),
                    ..Default::default()
                },
            };

            let response = self
                .llm
                .complete(request)
                .await
                .map_err(|e| TurnError::LlmError(e.to_string()))?;

            match response.finish_reason {
                FinishReason::Stop => {
                    self.message_store
                        .save_message(&session_id, "assistant", &response.content)
                        .await
                        .map_err(store_err)?;
                    let _ = self.compaction.compact_if_needed(&session_id).await;
                    return Ok(TurnOutput {
                        session_id,
                        assistant_text: response.content,
                        finish_reason: TurnFinishReason::Stop,
                        tool_iterations: iteration,
                    });
                }
                FinishReason::Length => {
                    self.message_store
                        .save_message(&session_id, "assistant", &response.content)
                        .await
                        .map_err(store_err)?;
                    let _ = self.compaction.compact_if_needed(&session_id).await;
                    return Ok(TurnOutput {
                        session_id,
                        assistant_text: response.content,
                        finish_reason: TurnFinishReason::LengthTruncated,
                        tool_iterations: iteration,
                    });
                }
                FinishReason::ToolCalls => {
                    if iteration >= self.config.max_tool_iterations {
                        return Err(TurnError::ToolLoopExceeded {
                            max: self.config.max_tool_iterations,
                        });
                    }

                    self.save_assistant_tool_call_message(&session_id, &response.tool_calls)
                        .await?;
                    messages.push(ChatMessage {
                        role: "assistant".to_string(),
                        content: response.content.clone(),
                        tool_calls: Some(
                            response
                                .tool_calls
                                .clone()
                                .into_iter()
                                .map(to_domain_call)
                                .collect(),
                        ),
                        tool_call_id: None,
                    });

                    self.execute_tool_calls(&session_id, &mut messages, &response)
                        .await?;
                    iteration = iteration.saturating_add(1);
                }
                FinishReason::ContentFilter | FinishReason::Error => {
                    self.message_store
                        .save_message(&session_id, "assistant", &response.content)
                        .await
                        .map_err(store_err)?;
                    let _ = self.compaction.compact_if_needed(&session_id).await;
                    return Ok(TurnOutput {
                        session_id,
                        assistant_text: response.content,
                        finish_reason: TurnFinishReason::Stop,
                        tool_iterations: iteration,
                    });
                }
            }
        }
    }

    async fn resolve_session_id(
        &self,
        user_id: &str,
        session_id: Option<&str>,
    ) -> Result<String, TurnError> {
        match session_id {
            Some(id) if !id.trim().is_empty() => Ok(id.to_string()),
            _ => self
                .message_store
                .get_or_create_default_session(user_id)
                .await
                .map_err(store_err),
        }
    }

    async fn save_assistant_tool_call_message(
        &self,
        session_id: &str,
        tool_calls: &[agent_llm::types::ToolCall],
    ) -> Result<(), TurnError> {
        let message = ChatMessage {
            role: "assistant".to_string(),
            content: String::new(),
            tool_calls: Some(tool_calls.iter().cloned().map(to_domain_call).collect()),
            tool_call_id: None,
        };
        self.message_store
            .save_message_ext(session_id, &message)
            .await
            .map_err(store_err)?;
        Ok(())
    }

    async fn execute_tool_calls(
        &self,
        session_id: &str,
        messages: &mut Vec<ChatMessage>,
        response: &LlmResponse,
    ) -> Result<(), TurnError> {
        let calls = response
            .tool_calls
            .iter()
            .map(llm_tool_call_to_tool_input)
            .collect::<Vec<_>>();
        let results = self.tools.execute_calls(calls).await;

        for result in results {
            let message = match result.result {
                Ok(output) => tool_output_to_chat_message(&result.request_id, &output),
                Err(err) => tool_output_to_chat_message(
                    &result.request_id,
                    &agent_tools::ToolOutput {
                        content_json: tool_error_json(&err),
                    },
                ),
            };
            self.message_store
                .save_message_ext(session_id, &message)
                .await
                .map_err(store_err)?;
            messages.push(message);
        }
        Ok(())
    }
}

fn store_err(error: StoreError) -> TurnError {
    TurnError::SessionError(format!("{error:?}"))
}

#[cfg(test)]
#[path = "turn_executor_tests.rs"]
mod tests;
