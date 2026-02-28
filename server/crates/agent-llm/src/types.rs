use std::time::Duration;

#[derive(Debug, Clone, PartialEq)]
pub struct LlmRequest {
    pub messages: Vec<ModelMessage>,
    pub tool_specs: Vec<ToolSpec>,
    pub builtin_tools: Vec<String>,
    pub previous_response_id: Option<String>,
    pub config: LlmRequestConfig,
    pub metadata: LlmRequestMetadata,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelMessage {
    pub role: String,
    pub content: String,
    pub tool_calls: Option<Vec<ToolCall>>,
    pub tool_call_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LlmRequestConfig {
    pub model: String,
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
    pub max_tokens: Option<u32>,
    pub timeout: Option<Duration>,
    pub json_mode: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LlmRequestMetadata {
    pub tenant_id: Option<String>,
    pub session_id: Option<String>,
    pub turn_id: Option<String>,
    pub trace_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LlmResponse {
    pub content: String,
    pub tool_calls: Vec<ToolCall>,
    pub finish_reason: FinishReason,
    pub usage: Option<LlmUsage>,
    pub model: String,
    pub provider: String,
    pub response_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ToolCall {
    pub call_id: String,
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ToolSpec {
    pub name: String,
    pub description: String,
    pub parameters_schema: serde_json::Value,
    pub strict: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FinishReason {
    Stop,
    ToolCalls,
    Length,
    ContentFilter,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LlmUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
    pub cache_read_tokens: Option<u32>,
    pub cache_write_tokens: Option<u32>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LlmProviderCapabilities {
    pub supports_stream: bool,
    pub supports_tools: bool,
    pub supports_vision: bool,
    pub supports_json_mode: bool,
    pub supports_stateful: bool,
}
