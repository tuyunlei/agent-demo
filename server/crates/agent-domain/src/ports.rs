#[async_trait::async_trait]
pub trait AuthPort: Send + Sync {
    async fn authenticate(&self, email: &str, password: &str) -> Result<AuthResult, AuthError>;

    async fn create_user(
        &self,
        email: &str,
        password: &str,
        display_name: &str,
    ) -> Result<AuthResult, AuthError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthResult {
    pub user_id: String,
    pub display_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthError {
    InvalidCredentials,
    AlreadyExists(String),
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
    pub tools: Vec<ToolSpec>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
    pub tool_calls: Option<Vec<ToolCall>>,
    pub tool_call_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ToolSpec {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ToolCall {
    pub call_id: String,
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolResult {
    pub call_id: String,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LlmResponse {
    pub content: String,
    pub model: String,
    pub usage: Option<LlmUsage>,
    pub tool_calls: Vec<ToolCall>,
    pub finish_reason: FinishReason,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FinishReason {
    Stop,
    ToolCalls,
    Length,
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

#[async_trait::async_trait]
pub trait ToolRuntime: Send + Sync {
    fn list_tools(&self) -> Vec<ToolSpec>;
    async fn execute(&self, name: &str, arguments: &str) -> Result<ToolResult, AgentError>;
}

#[async_trait::async_trait]
pub trait MessageStore: Send + Sync {
    async fn create_session(&self, user_id: &str, agent_id: &str) -> Result<String, StoreError>;
    async fn create_session_with_title(
        &self,
        user_id: &str,
        _title: &str,
    ) -> Result<StoredSession, StoreError> {
        let session_id = self.create_session(user_id, "").await?;
        self.get_session(user_id, &session_id)
            .await?
            .ok_or_else(|| StoreError::NotFound("session not found".to_string()))
    }
    async fn list_sessions(&self, _user_id: &str) -> Result<Vec<StoredSession>, StoreError> {
        Ok(Vec::new())
    }
    async fn get_session(
        &self,
        _user_id: &str,
        _session_id: &str,
    ) -> Result<Option<StoredSession>, StoreError> {
        Ok(None)
    }
    async fn save_message(
        &self,
        session_id: &str,
        role: &str,
        content: &str,
    ) -> Result<String, StoreError>;
    async fn save_message_ext(
        &self,
        session_id: &str,
        message: &ChatMessage,
    ) -> Result<String, StoreError> {
        self.save_message(session_id, &message.role, &message.content)
            .await
    }
    async fn get_session_messages(
        &self,
        session_id: &str,
        limit: i64,
    ) -> Result<Vec<StoredMessage>, StoreError>;
    async fn get_or_create_default_session(&self, user_id: &str) -> Result<String, StoreError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredMessage {
    pub id: String,
    pub session_id: String,
    pub role: String,
    pub content: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredSession {
    pub id: String,
    pub user_id: String,
    pub agent_id: String,
    pub title: String,
    pub summary: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub last_message_at: Option<i64>,
    pub archived: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoreError {
    NotFound(String),
    Internal(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentError {
    InvalidInput(String),
    Llm(LlmError),
    Store(StoreError),
}

impl From<LlmError> for AgentError {
    fn from(value: LlmError) -> Self {
        Self::Llm(value)
    }
}

impl From<StoreError> for AgentError {
    fn from(value: StoreError) -> Self {
        Self::Store(value)
    }
}

#[cfg(test)]
#[path = "ports_tests.rs"]
mod tests;
