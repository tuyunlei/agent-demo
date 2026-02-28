use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use agent_domain as domain;

use crate::error::LlmError;
use crate::traits::LlmProvider;
use crate::types::{
    FinishReason, LlmRequest, LlmRequestConfig, LlmRequestMetadata, LlmResponse, LlmUsage,
};

#[derive(Clone)]
pub struct MockLlmProvider {
    response: Arc<Mutex<Result<LlmResponse, LlmError>>>,
    sequence: Arc<Mutex<VecDeque<Result<LlmResponse, LlmError>>>>,
    calls: Arc<Mutex<Vec<LlmRequest>>>,
}

impl MockLlmProvider {
    pub fn new(response: Result<LlmResponse, LlmError>) -> Self {
        Self {
            response: Arc::new(Mutex::new(response)),
            sequence: Arc::new(Mutex::new(VecDeque::new())),
            calls: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn with_tool_call_sequence(responses: Vec<Result<LlmResponse, LlmError>>) -> Self {
        Self {
            response: Arc::new(Mutex::new(Ok(default_response("mock")))),
            sequence: Arc::new(Mutex::new(VecDeque::from(responses))),
            calls: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn with_text(text: &str) -> Self {
        Self::new(Ok(default_response(text)))
    }

    pub fn calls(&self) -> Vec<LlmRequest> {
        self.calls
            .lock()
            .expect("calls mutex should not be poisoned")
            .clone()
    }

    pub fn set_response(&self, response: Result<LlmResponse, LlmError>) {
        *self
            .response
            .lock()
            .expect("response mutex should not be poisoned") = response;
    }
}

fn default_response(text: &str) -> LlmResponse {
    LlmResponse {
        content: text.to_string(),
        model: "mock".to_string(),
        usage: Some(LlmUsage {
            prompt_tokens: 0,
            completion_tokens: 0,
            total_tokens: 0,
            cache_read_tokens: None,
            cache_write_tokens: None,
        }),
        tool_calls: Vec::new(),
        finish_reason: FinishReason::Stop,
        provider: "mock".to_string(),
        response_id: None,
    }
}

#[async_trait::async_trait]
impl LlmProvider for MockLlmProvider {
    fn provider_id(&self) -> &str {
        "mock"
    }

    async fn complete(&self, request: LlmRequest) -> Result<LlmResponse, LlmError> {
        self.calls
            .lock()
            .expect("calls mutex should not be poisoned")
            .push(request);

        if let Some(response) = self
            .sequence
            .lock()
            .expect("sequence mutex should not be poisoned")
            .pop_front()
        {
            return response;
        }

        self.response
            .lock()
            .expect("response mutex should not be poisoned")
            .clone()
    }
}

#[async_trait::async_trait]
impl domain::LlmProvider for MockLlmProvider {
    #[allow(clippy::too_many_lines)] // Test mock — splitting would reduce readability.
    async fn generate(
        &self,
        request: domain::LlmRequest,
    ) -> Result<domain::LlmResponse, domain::LlmError> {
        let req = LlmRequest {
            messages: request
                .messages
                .into_iter()
                .map(|m| crate::types::ModelMessage {
                    role: m.role,
                    content: m.content,
                    tool_calls: m.tool_calls.map(|calls| {
                        calls
                            .into_iter()
                            .map(|c| crate::types::ToolCall {
                                call_id: c.call_id,
                                name: c.name,
                                arguments: c.arguments,
                            })
                            .collect()
                    }),
                    tool_call_id: m.tool_call_id,
                })
                .collect(),
            tool_specs: request
                .tools
                .into_iter()
                .map(|t| crate::types::ToolSpec {
                    name: t.name,
                    description: t.description,
                    parameters_schema: t.parameters,
                    strict: false,
                })
                .collect(),
            builtin_tools: vec![],
            previous_response_id: None,
            config: LlmRequestConfig {
                model: request.model.unwrap_or_else(|| "mock".to_string()),
                temperature: request.temperature,
                top_p: None,
                max_tokens: request.max_tokens,
                timeout: None,
                json_mode: false,
            },
            metadata: LlmRequestMetadata::default(),
        };

        let resp = <Self as LlmProvider>::complete(self, req)
            .await
            .map_err(|e| match e {
                LlmError::RateLimit => domain::LlmError::RateLimited,
                LlmError::Timeout => domain::LlmError::Timeout,
                LlmError::InvalidRequest(m) => domain::LlmError::InvalidRequest(m),
                _ => domain::LlmError::ProviderError(e.to_string()),
            })?;

        Ok(domain::LlmResponse {
            content: resp.content,
            model: resp.model,
            usage: resp.usage.map(|u| domain::LlmUsage {
                input_tokens: u.prompt_tokens,
                output_tokens: u.completion_tokens,
                total_tokens: u.total_tokens,
            }),
            tool_calls: resp
                .tool_calls
                .into_iter()
                .map(|c| domain::ToolCall {
                    call_id: c.call_id,
                    name: c.name,
                    arguments: c.arguments,
                })
                .collect(),
            finish_reason: match resp.finish_reason {
                FinishReason::ToolCalls => domain::FinishReason::ToolCalls,
                FinishReason::Length => domain::FinishReason::Length,
                _ => domain::FinishReason::Stop,
            },
        })
    }
}
