use agent_context::{PromptSectionContext, RuntimeInfo, ToolPromptSpec};
use agent_domain::ChatMessage;
use agent_llm::types::{ModelMessage, ToolCall as LlmToolCall};
use agent_tools::{ToolError, ToolInput, ToolOutput};
use chrono::{DateTime, TimeZone, Utc};
use chrono_tz::Tz;

#[must_use]
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

#[must_use]
pub fn llm_tool_call_to_tool_input(call: &LlmToolCall) -> ToolInput {
    ToolInput {
        request_id: call.call_id.clone(),
        tool_name: call.name.clone(),
        arguments_json: call.arguments.clone(),
        timeout_ms: None,
    }
}

#[must_use]
pub fn tool_spec_to_llm_spec(spec: &agent_tools::ToolSpec) -> agent_llm::types::ToolSpec {
    agent_llm::types::ToolSpec {
        name: spec.name.clone(),
        description: spec.description.clone(),
        parameters_schema: spec.parameters_schema.clone(),
        strict: spec.strict,
    }
}

#[must_use]
pub fn tool_output_to_chat_message(call_id: &str, output: &ToolOutput) -> ChatMessage {
    ChatMessage {
        role: "tool".to_string(),
        content: output.content_json.clone(),
        tool_calls: None,
        tool_call_id: Some(call_id.to_string()),
    }
}

#[must_use]
pub fn tool_error_json(err: &ToolError) -> String {
    serde_json::json!({ "error": err.to_string() }).to_string()
}

#[must_use]
pub fn parse_timezone(name: &str) -> Tz {
    name.parse().unwrap_or(chrono_tz::UTC)
}

#[must_use]
pub fn format_message_content(role: &str, content: &str, created_at: i64, timezone: Tz) -> String {
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

#[must_use]
pub fn build_prompt_section_context(
    user_id: &str,
    session_id: &str,
    timezone: &str,
    tool_specs: &[agent_tools::ToolSpec],
) -> PromptSectionContext {
    PromptSectionContext {
        tenant_id: "default-tenant".to_string(),
        user_id: user_id.to_string(),
        agent_id: "assistant".to_string(),
        session_id: session_id.to_string(),
        timezone: timezone.to_string(),
        tools: tool_specs
            .iter()
            .map(|tool| ToolPromptSpec {
                name: tool.name.clone(),
                description: tool.description.clone(),
            })
            .collect(),
        now: Utc::now(),
        runtime_info: RuntimeInfo {
            model: "mock".to_string(),
            os: std::env::consts::OS.to_string(),
        },
    }
}
