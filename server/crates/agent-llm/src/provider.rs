use agent_domain as domain;
use reqwest::StatusCode;

use crate::error::LlmError;
use crate::provider_compat::{
    map_from_domain_request, map_to_domain_error, map_to_domain_response,
};
use crate::provider_wire::*;
use crate::traits::LlmProvider;
use crate::types::{
    FinishReason, LlmProviderCapabilities, LlmRequest, LlmResponse, LlmUsage, ToolCall,
};

pub struct OpenAiProvider {
    client: reqwest::Client,
    api_key: String,
    base_url: String,
    default_model: String,
}

impl OpenAiProvider {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self::with_config(api_key, "https://api.openai.com/v1", "gpt-4o-mini")
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
        let model = self.resolve_model(request.config.model.clone());
        let messages = Self::map_messages(request.messages);
        let tools = Self::map_tools(request.tool_specs);

        OpenAiChatCompletionRequest {
            model,
            messages,
            temperature: request.config.temperature,
            top_p: request.config.top_p,
            max_tokens: request.config.max_tokens,
            tools,
            response_format: request
                .config
                .json_mode
                .then_some(OpenAiResponseFormat::json_object()),
        }
    }

    fn resolve_model(&self, model: String) -> String {
        if model.is_empty() {
            self.default_model.clone()
        } else {
            model
        }
    }

    fn map_messages(messages: Vec<crate::types::ModelMessage>) -> Vec<OpenAiMessage> {
        messages
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
            .collect()
    }

    fn map_tools(tool_specs: Vec<crate::types::ToolSpec>) -> Option<Vec<OpenAiToolSpec>> {
        (!tool_specs.is_empty()).then(|| {
            tool_specs
                .into_iter()
                .map(|tool| OpenAiToolSpec {
                    tool_type: "function".to_string(),
                    function: OpenAiFunctionSpec {
                        name: tool.name,
                        description: tool.description,
                        parameters: tool.parameters_schema,
                        strict: tool.strict,
                    },
                })
                .collect()
        })
    }

    fn parse_success_body(body: &str) -> Result<LlmResponse, LlmError> {
        let parsed: OpenAiChatCompletionResponse =
            serde_json::from_str(body).map_err(|e| LlmError::InvalidResponse(e.to_string()))?;
        let choice = parsed
            .choices
            .into_iter()
            .next()
            .ok_or_else(|| LlmError::InvalidResponse("missing choices[0]".into()))?;

        Ok(LlmResponse {
            content: choice.message.content.unwrap_or_default(),
            tool_calls: parse_tool_calls(choice.message.tool_calls),
            finish_reason: parse_finish_reason(choice.finish_reason),
            usage: parsed.usage.map(|u| LlmUsage {
                prompt_tokens: u.prompt_tokens,
                completion_tokens: u.completion_tokens,
                total_tokens: u.total_tokens,
                cache_read_tokens: None,
                cache_write_tokens: None,
            }),
            model: parsed.model,
            provider: "openai-compatible".to_string(),
            response_id: parsed.id,
        })
    }

    fn map_http_error(status: StatusCode, body: String) -> LlmError {
        if status == StatusCode::TOO_MANY_REQUESTS {
            return LlmError::RateLimit;
        }
        if matches!(status, StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN) {
            return LlmError::AuthError;
        }
        if status == StatusCode::REQUEST_TIMEOUT {
            return LlmError::Timeout;
        }
        if status.is_client_error() {
            return LlmError::InvalidRequest(body);
        }
        if status.is_server_error() {
            return LlmError::ProviderDown;
        }
        LlmError::Transport(format!("unexpected status {status}: {body}"))
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
        Some("content_filter") => FinishReason::ContentFilter,
        Some("error") => FinishReason::Error,
        _ => FinishReason::Stop,
    }
}

#[async_trait::async_trait]
impl LlmProvider for OpenAiProvider {
    fn provider_id(&self) -> &str {
        "openai-compatible"
    }

    async fn complete(&self, request: LlmRequest) -> Result<LlmResponse, LlmError> {
        let timeout = request.config.timeout;
        let mut req = self
            .client
            .post(self.endpoint())
            .bearer_auth(&self.api_key)
            .json(&self.build_request_body(request));
        if let Some(timeout) = timeout {
            req = req.timeout(timeout);
        }

        let response = req.send().await.map_err(|e| {
            if e.is_timeout() {
                LlmError::Timeout
            } else {
                LlmError::Transport(e.to_string())
            }
        })?;
        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|e| LlmError::Transport(e.to_string()))?;
        if !status.is_success() {
            return Err(Self::map_http_error(status, body));
        }
        Self::parse_success_body(&body)
    }

    fn capabilities(&self) -> LlmProviderCapabilities {
        LlmProviderCapabilities {
            supports_stream: false,
            supports_tools: true,
            supports_vision: false,
            supports_json_mode: true,
            supports_stateful: false,
        }
    }
}

#[async_trait::async_trait]
impl domain::LlmProvider for OpenAiProvider {
    async fn generate(
        &self,
        request: domain::LlmRequest,
    ) -> Result<domain::LlmResponse, domain::LlmError> {
        let req = map_from_domain_request(request, &self.default_model);
        let resp = <Self as LlmProvider>::complete(self, req)
            .await
            .map_err(map_to_domain_error)?;
        Ok(map_to_domain_response(resp))
    }
}

#[cfg(test)]
#[path = "provider_tests.rs"]
mod tests;
