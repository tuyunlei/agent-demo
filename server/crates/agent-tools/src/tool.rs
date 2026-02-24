use async_trait::async_trait;

use crate::{ToolError, ToolSpec};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolInput {
    pub request_id: String,
    pub tool_name: String,
    pub arguments_json: String,
    pub timeout_ms: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolOutput {
    pub content_json: String,
}

#[async_trait]
pub trait Tool: Send + Sync {
    fn spec(&self) -> ToolSpec;
    async fn execute(&self, input: ToolInput) -> Result<ToolOutput, ToolError>;
}
