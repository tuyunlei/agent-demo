#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
pub enum LlmError {
    #[error("rate limited")]
    RateLimit,
    #[error("timeout")]
    Timeout,
    #[error("authentication failed")]
    AuthError,
    #[error("provider unavailable")]
    ProviderDown,
    #[error("invalid request: {0}")]
    InvalidRequest(String),
    #[error("invalid response: {0}")]
    InvalidResponse(String),
    #[error("unsupported capability: provider={provider} capability={capability}")]
    UnsupportedCapability {
        provider: String,
        capability: String,
    },
    #[error("transport error: {0}")]
    Transport(String),
    #[error("internal: {0}")]
    Internal(String),
}

impl LlmError {
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::RateLimit | Self::Timeout | Self::ProviderDown | Self::Transport(_)
        )
    }

    pub fn is_fallbackable(&self) -> bool {
        matches!(
            self,
            Self::RateLimit | Self::Timeout | Self::ProviderDown | Self::Transport(_)
        )
    }
}
