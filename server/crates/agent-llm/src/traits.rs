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
