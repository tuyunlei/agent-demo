use agent_domain::{ChatMessage, ToolSpec as DomainToolSpec};
use agent_llm::types::{ModelMessage, ToolCall as LlmToolCall, ToolSpec as LlmToolSpec};
use agent_tools::{ToolInput, ToolOutput};

pub fn chat_message_to_model_message(msg: &ChatMessage) -> ModelMessage {
    ModelMessage {
        role: msg.role.clone(),
        content: msg.content.clone(),
        tool_calls: msg.tool_calls.clone().map(|calls| {
            calls
                .into_iter()
                .map(|call| LlmToolCall {
                    call_id: call.call_id,
                    name: call.name,
                    arguments: call.arguments,
                })
                .collect()
        }),
        tool_call_id: msg.tool_call_id.clone(),
    }
}

pub fn llm_tool_call_to_tool_input(call: &LlmToolCall) -> ToolInput {
    ToolInput {
        request_id: call.call_id.clone(),
        tool_name: call.name.clone(),
        arguments_json: call.arguments.clone(),
        timeout_ms: None,
    }
}

pub fn tool_output_to_chat_message(call_id: &str, output: &ToolOutput) -> ChatMessage {
    ChatMessage {
        role: "tool".to_string(),
        content: output.content_json.clone(),
        tool_calls: None,
        tool_call_id: Some(call_id.to_string()),
    }
}

pub fn domain_tool_spec_to_llm_spec(spec: &DomainToolSpec) -> LlmToolSpec {
    LlmToolSpec {
        name: spec.name.clone(),
        description: spec.description.clone(),
        parameters_schema: spec.parameters.clone(),
        strict: false,
    }
}
