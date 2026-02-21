#[async_trait::async_trait]
pub trait AuthPort: Send + Sync {
    async fn authenticate(&self, email: &str, password: &str) -> Result<AuthResult, AuthError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthResult {
    pub user_id: String,
    pub display_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthError {
    InvalidCredentials,
    Internal(String),
}

#[async_trait::async_trait]
pub trait LlmProvider: Send + Sync {
    async fn generate(&self, request: LlmRequest) -> Result<LlmResponse, LlmError>;
}

#[derive(Debug, Clone, PartialEq)]
pub struct LlmRequest {
    pub messages: Vec<ChatMessage>,
    pub model: Option<String>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LlmResponse {
    pub content: String,
    pub model: String,
    pub usage: Option<LlmUsage>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LlmUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub total_tokens: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LlmError {
    ProviderError(String),
    RateLimited,
    InvalidRequest(String),
    Timeout,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentError {
    InvalidInput(String),
    Llm(LlmError),
}

impl From<LlmError> for AgentError {
    fn from(value: LlmError) -> Self {
        Self::Llm(value)
    }
}

#[cfg(test)]
mod tests {
    use super::{AgentError, AuthError, LlmError};

    #[test]
    fn auth_error_variants() {
        let invalid = AuthError::InvalidCredentials;
        let internal = AuthError::Internal("db down".to_string());

        assert_eq!(invalid, AuthError::InvalidCredentials);
        assert_eq!(internal, AuthError::Internal("db down".to_string()));
        assert!(format!("{internal:?}").contains("db down"));
    }

    #[test]
    fn llm_error_variants() {
        let provider = LlmError::ProviderError("bad response".to_string());
        let invalid = LlmError::InvalidRequest("missing messages".to_string());

        assert_eq!(LlmError::RateLimited, LlmError::RateLimited);
        assert_eq!(
            provider,
            LlmError::ProviderError("bad response".to_string())
        );
        assert_eq!(
            invalid,
            LlmError::InvalidRequest("missing messages".to_string())
        );
        assert!(format!("{:?}", LlmError::Timeout).contains("Timeout"));
    }

    #[test]
    fn agent_error_from_llm_error() {
        let err = AgentError::from(LlmError::RateLimited);

        assert_eq!(err, AgentError::Llm(LlmError::RateLimited));
    }
}
