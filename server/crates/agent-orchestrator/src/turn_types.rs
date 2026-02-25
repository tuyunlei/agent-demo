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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn turn_executor_config_default_values() {
        let cfg = TurnExecutorConfig::default();
        assert_eq!(cfg.max_tool_iterations, 10);
        assert_eq!(cfg.timezone, "Asia/Shanghai");
    }

    #[test]
    fn turn_input_output_construct() {
        let input = TurnInput {
            user_id: "u1".to_string(),
            session_id: Some("s1".to_string()),
            user_message: "hello".to_string(),
        };
        assert_eq!(input.session_id.as_deref(), Some("s1"));

        let output = TurnOutput {
            session_id: "s1".to_string(),
            assistant_text: "hi".to_string(),
            finish_reason: TurnFinishReason::Stop,
            tool_iterations: 2,
        };
        assert_eq!(output.finish_reason, TurnFinishReason::Stop);
        assert_eq!(output.tool_iterations, 2);
    }

    #[test]
    fn turn_error_display_variants() {
        assert_eq!(
            TurnError::InvalidInput("x".to_string()).to_string(),
            "invalid input: x"
        );
        assert_eq!(
            TurnError::SessionError("x".to_string()).to_string(),
            "session error: x"
        );
        assert_eq!(
            TurnError::LlmError("x".to_string()).to_string(),
            "llm error: x"
        );
        assert_eq!(
            TurnError::ToolLoopExceeded { max: 3 }.to_string(),
            "tool loop exceeded max iterations: 3"
        );
        assert_eq!(
            TurnError::Internal("x".to_string()).to_string(),
            "internal error: x"
        );
    }
}
