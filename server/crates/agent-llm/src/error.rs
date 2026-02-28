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
    #[must_use]
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::RateLimit | Self::Timeout | Self::ProviderDown | Self::Transport(_)
        )
    }

    #[must_use]
    pub fn is_fallbackable(&self) -> bool {
        matches!(
            self,
            Self::RateLimit | Self::Timeout | Self::ProviderDown | Self::Transport(_)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::LlmError;

    #[test]
    fn is_retryable_matches_expected_variants() {
        let retryable = [
            LlmError::RateLimit,
            LlmError::Timeout,
            LlmError::ProviderDown,
            LlmError::Transport("io".into()),
        ];
        for err in retryable {
            assert!(err.is_retryable());
        }

        let non_retryable = [
            LlmError::AuthError,
            LlmError::InvalidRequest("bad".into()),
            LlmError::InvalidResponse("bad".into()),
            LlmError::UnsupportedCapability {
                provider: "p".into(),
                capability: "stream".into(),
            },
            LlmError::Internal("bug".into()),
        ];
        for err in non_retryable {
            assert!(!err.is_retryable());
        }
    }

    #[test]
    fn is_fallbackable_matches_expected_variants() {
        let fallbackable = [
            LlmError::RateLimit,
            LlmError::Timeout,
            LlmError::ProviderDown,
            LlmError::Transport("io".into()),
        ];
        for err in fallbackable {
            assert!(err.is_fallbackable());
        }

        let non_fallbackable = [
            LlmError::AuthError,
            LlmError::InvalidRequest("bad".into()),
            LlmError::InvalidResponse("bad".into()),
            LlmError::UnsupportedCapability {
                provider: "p".into(),
                capability: "stream".into(),
            },
            LlmError::Internal("bug".into()),
        ];
        for err in non_fallbackable {
            assert!(!err.is_fallbackable());
        }
    }
}
