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
