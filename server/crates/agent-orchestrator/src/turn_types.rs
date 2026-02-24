use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TurnExecutorConfig {
    pub max_tool_iterations: u8,
    pub timezone: String,
}

impl Default for TurnExecutorConfig {
    fn default() -> Self {
        Self {
            max_tool_iterations: 10,
            timezone: "Asia/Shanghai".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TurnInput {
    pub user_id: String,
    pub session_id: Option<String>,
    pub user_message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TurnOutput {
    pub session_id: String,
    pub assistant_text: String,
    pub finish_reason: TurnFinishReason,
    pub tool_iterations: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TurnFinishReason {
    Stop,
    ToolLoopExceeded,
    LengthTruncated,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TurnError {
    InvalidInput(String),
    SessionError(String),
    LlmError(String),
    ToolLoopExceeded { max: u8 },
    Internal(String),
}

impl fmt::Display for TurnError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidInput(msg) => write!(f, "invalid input: {msg}"),
            Self::SessionError(msg) => write!(f, "session error: {msg}"),
            Self::LlmError(msg) => write!(f, "llm error: {msg}"),
            Self::ToolLoopExceeded { max } => write!(f, "tool loop exceeded max iterations: {max}"),
            Self::Internal(msg) => write!(f, "internal error: {msg}"),
        }
    }
}

impl std::error::Error for TurnError {}
