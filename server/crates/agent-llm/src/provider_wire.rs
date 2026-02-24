use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct OpenAiMessage {
    pub role: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<OpenAiToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct OpenAiChatCompletionRequest {
    pub model: String,
    pub messages: Vec<OpenAiMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<OpenAiToolSpec>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<OpenAiResponseFormat>,
}

#[derive(Debug, Serialize)]
pub struct OpenAiToolSpec {
    #[serde(rename = "type")]
    pub tool_type: String,
    pub function: OpenAiFunctionSpec,
}

#[derive(Debug, Serialize)]
pub struct OpenAiFunctionSpec {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
    pub strict: bool,
}

#[derive(Debug, Serialize)]
pub struct OpenAiResponseFormat {
    #[serde(rename = "type")]
    pub format_type: String,
}

impl OpenAiResponseFormat {
    pub fn json_object() -> Self {
        Self {
            format_type: "json_object".to_string(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OpenAiToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub call_type: String,
    pub function: OpenAiFunctionCall,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OpenAiFunctionCall {
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Deserialize)]
pub struct OpenAiChatCompletionResponse {
    pub id: Option<String>,
    pub model: String,
    pub choices: Vec<OpenAiChoice>,
    pub usage: Option<OpenAiUsage>,
}

#[derive(Debug, Deserialize)]
pub struct OpenAiChoice {
    pub message: OpenAiChoiceMessage,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct OpenAiChoiceMessage {
    pub content: Option<String>,
    pub tool_calls: Option<Vec<OpenAiToolCall>>,
}

#[derive(Debug, Deserialize)]
pub struct OpenAiUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}
