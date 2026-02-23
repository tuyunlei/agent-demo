use agent_domain::{
    FinishReason, LlmError, LlmProvider, LlmRequest, LlmResponse, LlmUsage, ToolCall,
};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};

pub struct OpenAiProvider {
    client: reqwest::Client,
    api_key: String,
    base_url: String,
    default_model: String,
}

impl OpenAiProvider {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key: api_key.into(),
            base_url: "https://api.openai.com/v1".to_string(),
            default_model: "gpt-4o-mini".to_string(),
        }
    }

    pub fn with_config(
        api_key: impl Into<String>,
        base_url: impl Into<String>,
        default_model: impl Into<String>,
    ) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key: api_key.into(),
            base_url: base_url.into(),
            default_model: default_model.into(),
        }
    }

    fn endpoint(&self) -> String {
        format!("{}/chat/completions", self.base_url.trim_end_matches('/'))
    }

    fn build_request_body(&self, request: LlmRequest) -> OpenAiChatCompletionRequest {
        OpenAiChatCompletionRequest {
            model: request.model.unwrap_or_else(|| self.default_model.clone()),
            messages: request
                .messages
                .into_iter()
                .map(|m| OpenAiMessage {
                    role: m.role,
                    content: m.content,
                    tool_calls: m.tool_calls.map(|calls| {
                        calls
                            .into_iter()
                            .map(|call| OpenAiToolCall {
                                id: call.call_id,
                                call_type: "function".to_string(),
                                function: OpenAiFunctionCall {
                                    name: call.name,
                                    arguments: call.arguments,
                                },
                            })
                            .collect()
                    }),
                    tool_call_id: m.tool_call_id,
                })
                .collect(),
            temperature: request.temperature,
            max_tokens: request.max_tokens,
            tools: (!request.tools.is_empty()).then(|| {
                request
                    .tools
                    .into_iter()
                    .map(|tool| OpenAiToolSpec {
                        tool_type: "function".to_string(),
                        function: OpenAiFunctionSpec {
                            name: tool.name,
                            description: tool.description,
                            parameters: tool.parameters,
                        },
                    })
                    .collect()
            }),
        }
    }

    fn parse_success_body(body: &str) -> Result<LlmResponse, LlmError> {
        let parsed: OpenAiChatCompletionResponse = serde_json::from_str(body)
            .map_err(|e| LlmError::ProviderError(format!("invalid response body: {e}")))?;

        let choice = parsed
            .choices
            .into_iter()
            .next()
            .ok_or_else(|| LlmError::ProviderError("missing choices[0]".into()))?;

        let usage = parsed.usage.map(|u| LlmUsage {
            input_tokens: u.prompt_tokens,
            output_tokens: u.completion_tokens,
            total_tokens: u.total_tokens,
        });

        Ok(LlmResponse {
            content: choice.message.content.unwrap_or_default(),
            model: parsed.model,
            usage,
            tool_calls: parse_tool_calls(choice.message.tool_calls),
            finish_reason: parse_finish_reason(choice.finish_reason),
        })
    }

    fn map_http_error(status: StatusCode, body: String) -> LlmError {
        if status == StatusCode::TOO_MANY_REQUESTS {
            return LlmError::RateLimited;
        }

        if status.is_client_error() {
            return LlmError::InvalidRequest(body);
        }

        if status.is_server_error() {
            return LlmError::ProviderError(body);
        }

        LlmError::ProviderError(format!("unexpected status: {status}"))
    }
}

fn parse_tool_calls(tool_calls: Option<Vec<OpenAiToolCall>>) -> Vec<ToolCall> {
    tool_calls
        .unwrap_or_default()
        .into_iter()
        .map(|call| ToolCall {
            call_id: call.id,
            name: call.function.name,
            arguments: call.function.arguments,
        })
        .collect()
}

fn parse_finish_reason(reason: Option<String>) -> FinishReason {
    match reason.as_deref() {
        Some("tool_calls") => FinishReason::ToolCalls,
        Some("length") => FinishReason::Length,
        _ => FinishReason::Stop,
    }
}

#[async_trait::async_trait]
impl LlmProvider for OpenAiProvider {
    async fn generate(&self, request: LlmRequest) -> Result<LlmResponse, LlmError> {
        let payload = self.build_request_body(request);
        let response = self
            .client
            .post(self.endpoint())
            .bearer_auth(&self.api_key)
            .json(&payload)
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    LlmError::Timeout
                } else {
                    LlmError::ProviderError(e.to_string())
                }
            })?;

        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|e| LlmError::ProviderError(e.to_string()))?;

        if !status.is_success() {
            return Err(Self::map_http_error(status, body));
        }

        Self::parse_success_body(&body)
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct OpenAiMessage {
    role: String,
    content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<OpenAiToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_call_id: Option<String>,
}

#[derive(Debug, Serialize)]
struct OpenAiChatCompletionRequest {
    model: String,
    messages: Vec<OpenAiMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<OpenAiToolSpec>>,
}

#[derive(Debug, Serialize)]
struct OpenAiToolSpec {
    #[serde(rename = "type")]
    tool_type: String,
    function: OpenAiFunctionSpec,
}

#[derive(Debug, Serialize)]
struct OpenAiFunctionSpec {
    name: String,
    description: String,
    parameters: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
struct OpenAiToolCall {
    id: String,
    #[serde(rename = "type")]
    call_type: String,
    function: OpenAiFunctionCall,
}

#[derive(Debug, Serialize, Deserialize)]
struct OpenAiFunctionCall {
    name: String,
    arguments: String,
}

#[derive(Debug, Deserialize)]
struct OpenAiChatCompletionResponse {
    model: String,
    choices: Vec<OpenAiChoice>,
    usage: Option<OpenAiUsage>,
}

#[derive(Debug, Deserialize)]
struct OpenAiChoice {
    message: OpenAiChoiceMessage,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAiChoiceMessage {
    content: Option<String>,
    tool_calls: Option<Vec<OpenAiToolCall>>,
}

#[derive(Debug, Deserialize)]
struct OpenAiUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

#[cfg(test)]
#[path = "provider_tests.rs"]
mod tests;
