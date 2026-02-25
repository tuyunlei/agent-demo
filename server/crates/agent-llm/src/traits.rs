use std::pin::Pin;

use futures_core::Stream;

use crate::error::LlmError;
use crate::types::{LlmProviderCapabilities, LlmRequest, LlmResponse};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LlmStreamChunk {
    pub delta_text: Option<String>,
}

pub type LlmStream = Pin<Box<dyn Stream<Item = Result<LlmStreamChunk, LlmError>> + Send>>;

#[async_trait::async_trait]
pub trait LlmProvider: Send + Sync {
    fn provider_id(&self) -> &str;

    async fn complete(&self, req: LlmRequest) -> Result<LlmResponse, LlmError>;

    async fn stream(&self, _req: LlmRequest) -> Result<LlmStream, LlmError> {
        Err(LlmError::UnsupportedCapability {
            provider: self.provider_id().to_string(),
            capability: "stream".to_string(),
        })
    }

    fn capabilities(&self) -> LlmProviderCapabilities {
        LlmProviderCapabilities::default()
    }
}

#[cfg(test)]
mod tests {
    use futures::executor::block_on;

    use super::*;
    use crate::types::{LlmRequestConfig, LlmRequestMetadata, ModelMessage};

    struct DummyProvider;

    #[async_trait::async_trait]
    impl LlmProvider for DummyProvider {
        fn provider_id(&self) -> &str {
            "dummy"
        }

        async fn complete(&self, _req: LlmRequest) -> Result<LlmResponse, LlmError> {
            Err(LlmError::Internal("not used".into()))
        }
    }

    fn sample_request() -> LlmRequest {
        LlmRequest {
            messages: vec![ModelMessage {
                role: "user".into(),
                content: "hello".into(),
                tool_calls: None,
                tool_call_id: None,
            }],
            tool_specs: vec![],
            config: LlmRequestConfig {
                model: String::new(),
                temperature: None,
                top_p: None,
                max_tokens: None,
                timeout: None,
                json_mode: false,
            },
            metadata: LlmRequestMetadata::default(),
        }
    }

    #[test]
    fn stream_default_impl_returns_unsupported_capability() {
        let provider = DummyProvider;
        let result = block_on(provider.stream(sample_request()));
        match result {
            Err(err) => {
                assert_eq!(
                    err,
                    LlmError::UnsupportedCapability {
                        provider: "dummy".into(),
                        capability: "stream".into(),
                    }
                );
            }
            Ok(_) => panic!("stream should fail"),
        }
    }
}
