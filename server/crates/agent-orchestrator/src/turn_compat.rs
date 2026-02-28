use agent_domain::{ChatMessage, ToolCall, ToolSpec as DomainToolSpec, ToolSpec};
use agent_llm::types::{ModelMessage, ToolCall as LlmToolCall, ToolSpec as LlmToolSpec};
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
pub fn tool_output_to_chat_message(call_id: &str, output: &ToolOutput) -> ChatMessage {
    ChatMessage {
        role: "tool".to_string(),
        content: output.content_json.clone(),
        tool_calls: None,
        tool_call_id: Some(call_id.to_string()),
    }
}

#[must_use]
pub fn domain_tool_spec_to_llm_spec(spec: &DomainToolSpec) -> LlmToolSpec {
    LlmToolSpec {
        name: spec.name.clone(),
        description: spec.description.clone(),
        parameters_schema: spec.parameters.clone(),
        strict: false,
    }
}

#[must_use]
pub fn to_domain_call(call: agent_llm::types::ToolCall) -> ToolCall {
    ToolCall {
        call_id: call.call_id,
        name: call.name,
        arguments: call.arguments,
    }
}

#[must_use]
pub fn tool_specs_to_domain_specs(specs: &[agent_tools::ToolSpec]) -> Vec<ToolSpec> {
    specs
        .iter()
        .map(|spec| ToolSpec {
            name: spec.name.clone(),
            description: spec.description.clone(),
            parameters: spec.parameters_schema.clone(),
        })
        .collect()
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
pub fn build_system_prompt(tool_specs: &[ToolSpec], timezone: Tz) -> String {
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
