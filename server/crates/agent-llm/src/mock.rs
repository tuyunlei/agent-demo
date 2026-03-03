use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use crate::error::LlmError;
use crate::traits::LlmProvider;
use crate::types::{FinishReason, LlmRequest, LlmResponse, LlmUsage};

#[derive(Clone)]
pub struct MockLlmProvider {
    response: Arc<Mutex<Result<LlmResponse, LlmError>>>,
    sequence: Arc<Mutex<VecDeque<Result<LlmResponse, LlmError>>>>,
    calls: Arc<Mutex<Vec<LlmRequest>>>,
}

impl MockLlmProvider {
    #[must_use]
    pub fn new(response: Result<LlmResponse, LlmError>) -> Self {
        Self {
            response: Arc::new(Mutex::new(response)),
            sequence: Arc::new(Mutex::new(VecDeque::new())),
            calls: Arc::new(Mutex::new(Vec::new())),
        }
    }

    #[must_use]
    pub fn with_tool_call_sequence(responses: Vec<Result<LlmResponse, LlmError>>) -> Self {
        Self {
            response: Arc::new(Mutex::new(Ok(default_response("mock")))),
            sequence: Arc::new(Mutex::new(VecDeque::from(responses))),
            calls: Arc::new(Mutex::new(Vec::new())),
        }
    }

    #[must_use]
    pub fn with_text(text: &str) -> Self {
        Self::new(Ok(default_response(text)))
    }

    #[must_use]
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
