use std::sync::Arc;

use agent_context::ContextBuilder;
use agent_domain::{ChatMessage, MessageStore, StoreError};
use agent_llm::LlmProvider;
use agent_llm::types::{
    FinishReason, LlmRequest, LlmRequestConfig, LlmRequestMetadata, LlmResponse,
};
use agent_memory::CompactionService;
use agent_tools::ToolRuntime;

use crate::chat_runtime::ChatRuntime;
use crate::turn_compat::{
    build_prompt_section_context, chat_message_to_model_message, format_message_content,
    parse_timezone, tool_spec_to_llm_spec,
};
use crate::turn_types::{TurnError, TurnExecutorConfig, TurnFinishReason, TurnInput, TurnOutput};

pub struct TurnExecutor {
    llm: Arc<dyn LlmProvider>,
    tools: Arc<dyn ToolRuntime>,
    message_store: Arc<dyn MessageStore>,
    compaction: Arc<dyn CompactionService>,
    context_builder: Arc<dyn ContextBuilder>,
    config: TurnExecutorConfig,
}

pub struct TurnExecutorDeps {
    pub llm: Arc<dyn LlmProvider>,
    pub tools: Arc<dyn ToolRuntime>,
    pub message_store: Arc<dyn MessageStore>,
    pub compaction: Arc<dyn CompactionService>,
    pub context_builder: Arc<dyn ContextBuilder>,
    pub config: TurnExecutorConfig,
}

impl TurnExecutor {
    #[must_use]
    pub fn new(deps: TurnExecutorDeps) -> Self {
        let TurnExecutorDeps {
            llm,
            tools,
            message_store,
            compaction,
            context_builder,
            config,
        } = deps;
        Self {
            llm,
            tools,
            message_store,
            compaction,
            context_builder,
            config,
        }
    }

    pub async fn run_turn(&self, input: TurnInput) -> Result<TurnOutput, TurnError> {
        self.validate_input(&input)?;

        let session_id = self
            .resolve_session_id(&input.user_id, input.session_id.as_deref())
            .await?;
        self.save_user_message(&session_id, &input.user_message)
            .await?;

        let timezone = parse_timezone(&self.config.timezone);
        let tool_specs = self.tools.list_specs();
        let mut messages = self
            .build_messages_with_history(&input.user_id, &session_id, timezone, &tool_specs)
            .await?;

        let mut iteration: u8 = 0;
        loop {
            let response = self
                .request_llm_response(&session_id, &messages, &tool_specs)
                .await?;

            if let Some(output) = self
                .finish_if_terminal(&session_id, &response, iteration)
                .await?
            {
                return Ok(output);
            }

            self.process_tool_call_iteration(&session_id, &mut messages, &response, &mut iteration)
                .await?;
        }
    }

    fn validate_input(&self, input: &TurnInput) -> Result<(), TurnError> {
        if input.user_message.trim().is_empty() {
            return Err(TurnError::InvalidInput(
                "message content cannot be empty".to_string(),
            ));
        }
        Ok(())
    }

    async fn save_user_message(
        &self,
        session_id: &str,
        user_message: &str,
    ) -> Result<(), TurnError> {
        self.message_store
            .save_message(session_id, "user", user_message)
            .await
            .map_err(store_err)
            .map(|_| ())
    }

    async fn build_messages_with_history(
        &self,
        user_id: &str,
        session_id: &str,
        timezone: chrono_tz::Tz,
        tool_specs: &[agent_tools::ToolSpec],
    ) -> Result<Vec<ChatMessage>, TurnError> {
        let history = self
            .message_store
            .get_session_messages(session_id, 50)
            .await
            .map_err(store_err)?;

        let prompt_ctx =
            build_prompt_section_context(user_id, session_id, &self.config.timezone, tool_specs);
        let system_prompt = self
            .context_builder
            .build_system_prompt(&prompt_ctx)
            .map_err(|err| TurnError::Internal(format!("context build failed: {err}")))?;
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

        Ok(messages)
    }

    async fn request_llm_response(
        &self,
        session_id: &str,
        messages: &[ChatMessage],
        tool_specs: &[agent_tools::ToolSpec],
    ) -> Result<LlmResponse, TurnError> {
        let request = LlmRequest {
            messages: messages
                .iter()
                .map(chat_message_to_model_message)
                .collect::<Vec<_>>(),
            tool_specs: tool_specs
                .iter()
                .map(tool_spec_to_llm_spec)
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
                session_id: Some(session_id.to_string()),
                ..Default::default()
            },
        };

        self.llm
            .complete(request)
            .await
            .map_err(|e| TurnError::LlmError(e.to_string()))
    }

    async fn finish_if_terminal(
        &self,
        session_id: &str,
        response: &LlmResponse,
        iteration: u8,
    ) -> Result<Option<TurnOutput>, TurnError> {
        let finish_reason = match response.finish_reason {
            FinishReason::Stop | FinishReason::ContentFilter | FinishReason::Error => {
                Some(TurnFinishReason::Stop)
            }
            FinishReason::Length => Some(TurnFinishReason::LengthTruncated),
            FinishReason::ToolCalls => None,
        };

        let Some(finish_reason) = finish_reason else {
            return Ok(None);
        };

        self.message_store
            .save_message(session_id, "assistant", &response.content)
            .await
            .map_err(store_err)?;
        let _ = self.compaction.compact_if_needed(session_id).await;

        Ok(Some(TurnOutput {
            session_id: session_id.to_string(),
            assistant_text: response.content.clone(),
            finish_reason,
            tool_iterations: iteration,
        }))
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
}

#[async_trait::async_trait]
impl ChatRuntime for TurnExecutor {
    async fn run_turn(&self, input: TurnInput) -> Result<TurnOutput, TurnError> {
        TurnExecutor::run_turn(self, input).await
    }
}

fn store_err(error: StoreError) -> TurnError {
    TurnError::SessionError(format!("{error:?}"))
}

#[path = "turn_executor_tool_loop.rs"]
mod turn_executor_tool_loop;

#[cfg(test)]
#[path = "turn_executor_tests.rs"]
mod tests;
