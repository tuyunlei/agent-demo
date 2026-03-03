use agent_domain::ChatMessage;
use agent_llm::types::LlmResponse;

use super::{TurnError, TurnExecutor, store_err};
use crate::turn_compat::{
    llm_tool_call_to_tool_input, tool_error_json, tool_output_to_chat_message,
};

impl TurnExecutor {
    pub(super) async fn process_tool_call_iteration(
        &self,
        session_id: &str,
        messages: &mut Vec<ChatMessage>,
        response: &LlmResponse,
        iteration: &mut u8,
    ) -> Result<(), TurnError> {
        if *iteration >= self.config.max_tool_iterations {
            return Err(TurnError::ToolLoopExceeded {
                max: self.config.max_tool_iterations,
            });
        }

        self.save_assistant_tool_call_message(session_id, &response.tool_calls)
            .await?;
        messages.push(ChatMessage {
            role: "assistant".to_string(),
            content: response.content.clone(),
            tool_calls: Some(
                response
                    .tool_calls
                    .clone()
                    .into_iter()
                    .map(|call| agent_domain::ToolCall {
                        call_id: call.call_id,
                        name: call.name,
                        arguments: call.arguments,
                    })
                    .collect(),
            ),
            tool_call_id: None,
        });

        self.execute_tool_calls(session_id, messages, response)
            .await?;
        *iteration = iteration.saturating_add(1);

        Ok(())
    }

    async fn save_assistant_tool_call_message(
        &self,
        session_id: &str,
        tool_calls: &[agent_llm::types::ToolCall],
    ) -> Result<(), TurnError> {
        let message = ChatMessage {
            role: "assistant".to_string(),
            content: String::new(),
            tool_calls: Some(
                tool_calls
                    .iter()
                    .cloned()
                    .map(|call| agent_domain::ToolCall {
                        call_id: call.call_id,
                        name: call.name,
                        arguments: call.arguments,
                    })
                    .collect(),
            ),
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
