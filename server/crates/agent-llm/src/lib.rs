mod provider_wire;

pub mod error;
pub mod mock;
pub mod provider;
pub mod traits;
pub mod types;

pub use error::LlmError;
pub use mock::MockLlmProvider;
pub use provider::OpenAiProvider;
pub use traits::{LlmProvider, LlmStream, LlmStreamChunk};
pub use types::{
    FinishReason, LlmProviderCapabilities, LlmRequest, LlmRequestConfig, LlmRequestMetadata,
    LlmResponse, LlmUsage, ModelMessage, ToolCall, ToolSpec,
};
