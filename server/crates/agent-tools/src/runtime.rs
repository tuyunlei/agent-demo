use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;

use crate::{Tool, ToolError, ToolInput, ToolOutput, ToolSpec};

/// Result of a single tool call within a batch (design doc §3.2).
#[derive(Debug)]
pub struct ToolCallResult {
    pub request_id: String,
    pub result: Result<ToolOutput, ToolError>,
}

#[async_trait]
pub trait ToolRuntime: Send + Sync {
    /// Register a tool (startup or hot-reload).
    fn register(&mut self, tool: Box<dyn Tool>);

    /// List specs of all currently visible tools.
    fn list_specs(&self) -> Vec<ToolSpec>;

    /// Execute a batch of tool calls, returning results in order (design doc §3.2).
    /// Each call is isolated: one failure does not affect others.
    async fn execute_calls(&self, calls: Vec<ToolInput>) -> Vec<ToolCallResult>;
}

#[derive(Default)]
pub struct DefaultToolRuntime {
    tools: HashMap<String, Arc<dyn Tool>>,
}

impl DefaultToolRuntime {
    pub fn new() -> Self {
        Self::default()
    }

    /// Execute a single tool call (convenience helper).
    pub async fn execute_call(&self, input: ToolInput) -> Result<ToolOutput, ToolError> {
        let tool = self
            .tools
            .get(&input.tool_name)
            .ok_or_else(|| ToolError::NotFound(input.tool_name.clone()))?;

        if let Some(timeout_ms) = input.timeout_ms {
            let run = tool.execute(input);
            match tokio::time::timeout(std::time::Duration::from_millis(u64::from(timeout_ms)), run)
                .await
            {
                Ok(result) => result,
                Err(_) => Err(ToolError::Timeout(timeout_ms)),
            }
        } else {
            tool.execute(input).await
        }
    }
}

#[async_trait]
impl ToolRuntime for DefaultToolRuntime {
    fn register(&mut self, tool: Box<dyn Tool>) {
        let name = tool.spec().name;
        self.tools.insert(name, Arc::from(tool));
    }

    fn list_specs(&self) -> Vec<ToolSpec> {
        self.tools.values().map(|tool| tool.spec()).collect()
    }

    async fn execute_calls(&self, calls: Vec<ToolInput>) -> Vec<ToolCallResult> {
        let mut results = Vec::with_capacity(calls.len());
        for call in calls {
            let request_id = call.request_id.clone();
            let result = self.execute_call(call).await;
            results.push(ToolCallResult { request_id, result });
        }
        results
    }
}

#[cfg(test)]
mod tests {
    use async_trait::async_trait;
    use serde_json::json;

    use super::*;

    struct EchoTool;

    #[async_trait]
    impl Tool for EchoTool {
        fn spec(&self) -> ToolSpec {
            ToolSpec {
                name: "echo".to_string(),
                description: "Echo input".to_string(),
                parameters_schema: json!({"type": "object"}),
                strict: false,
                execution_class: crate::ExecutionClass::Local,
            }
        }

        async fn execute(&self, input: ToolInput) -> Result<ToolOutput, ToolError> {
            Ok(ToolOutput {
                content_json: input.arguments_json,
            })
        }
    }

    #[tokio::test]
    async fn register_and_list_specs() {
        let mut runtime = DefaultToolRuntime::new();
        runtime.register(Box::new(EchoTool));

        let specs = runtime.list_specs();
        assert_eq!(specs.len(), 1);
        assert_eq!(specs[0].name, "echo");
    }

    #[tokio::test]
    async fn execute_calls_batch() {
        let mut runtime = DefaultToolRuntime::new();
        runtime.register(Box::new(EchoTool));

        let results = runtime
            .execute_calls(vec![
                ToolInput {
                    request_id: "r1".to_string(),
                    tool_name: "echo".to_string(),
                    arguments_json: r#"{"x":1}"#.to_string(),
                    timeout_ms: None,
                },
                ToolInput {
                    request_id: "r2".to_string(),
                    tool_name: "missing".to_string(),
                    arguments_json: "{}".to_string(),
                    timeout_ms: None,
                },
            ])
            .await;

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].request_id, "r1");
        assert!(results[0].result.is_ok());
        assert_eq!(results[1].request_id, "r2");
        assert!(results[1].result.is_err());
    }

    #[tokio::test]
    async fn execute_call_single() {
        let mut runtime = DefaultToolRuntime::new();
        runtime.register(Box::new(EchoTool));

        let output = runtime
            .execute_call(ToolInput {
                request_id: "r1".to_string(),
                tool_name: "echo".to_string(),
                arguments_json: r#"{"x":1}"#.to_string(),
                timeout_ms: None,
            })
            .await
            .expect("echo should execute");

        assert_eq!(output.content_json, r#"{"x":1}"#);
    }

    #[tokio::test]
    async fn execute_unregistered_tool_returns_not_found() {
        let runtime = DefaultToolRuntime::new();
        let error = runtime
            .execute_call(ToolInput {
                request_id: "r1".to_string(),
                tool_name: "missing".to_string(),
                arguments_json: "{}".to_string(),
                timeout_ms: None,
            })
            .await
            .expect_err("missing tool should fail");

        assert!(matches!(error, ToolError::NotFound(name) if name == "missing"));
    }
}
