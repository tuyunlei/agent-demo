#[derive(Debug, thiserror::Error)]
pub enum ToolError {
    #[error("invalid arguments: {0}")]
    InvalidArguments(String),
    #[error("execution failed: {0}")]
    ExecutionFailed(String),
    #[error("timeout after {0}ms")]
    Timeout(u32),
    #[error("tool not found: {0}")]
    NotFound(String),
}
