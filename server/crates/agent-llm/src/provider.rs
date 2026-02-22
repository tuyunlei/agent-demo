use agent_domain::{LlmError, LlmProvider, LlmRequest, LlmResponse, LlmUsage};
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
                })
                .collect(),
            temperature: request.temperature,
            max_tokens: request.max_tokens,
        }
    }

    fn parse_success_body(body: &str) -> Result<LlmResponse, LlmError> {
        let parsed: OpenAiChatCompletionResponse = serde_json::from_str(body)
            .map_err(|e| LlmError::ProviderError(format!("invalid response body: {e}")))?;

        let content = parsed
            .choices
            .into_iter()
            .next()
            .map(|c| c.message.content)
            .filter(|c| !c.is_empty())
            .ok_or_else(|| LlmError::ProviderError("missing choices[0].message.content".into()))?;

        let usage = parsed.usage.map(|u| LlmUsage {
            input_tokens: u.prompt_tokens,
            output_tokens: u.completion_tokens,
            total_tokens: u.total_tokens,
        });

        Ok(LlmResponse {
            content,
            model: parsed.model,
            usage,
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
}

#[derive(Debug, Serialize)]
struct OpenAiChatCompletionRequest {
    model: String,
    messages: Vec<OpenAiMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
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
}

#[derive(Debug, Deserialize)]
struct OpenAiChoiceMessage {
    content: String,
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
