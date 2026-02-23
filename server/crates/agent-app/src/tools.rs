use std::collections::HashMap;

use agent_domain::{AgentError, ToolResult, ToolRuntime, ToolSpec};
use chrono::Utc;
use chrono_tz::Tz;
use serde::Deserialize;
use serde_json::json;

#[derive(Default)]
pub struct BuiltinToolRuntime {
    specs: HashMap<String, ToolSpec>,
}

impl BuiltinToolRuntime {
    pub fn new() -> Self {
        let mut specs = HashMap::new();
        specs.insert(
            "get_current_time".to_string(),
            ToolSpec {
                name: "get_current_time".to_string(),
                description: "Get current time in an IANA timezone".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "timezone": {
                            "type": "string",
                            "description": "IANA timezone like Asia/Shanghai, defaults to UTC"
                        }
                    },
                    "required": []
                }),
            },
        );
        Self { specs }
    }
}

#[derive(Deserialize)]
struct GetCurrentTimeArgs {
    timezone: Option<String>,
}

#[async_trait::async_trait]
impl ToolRuntime for BuiltinToolRuntime {
    fn list_tools(&self) -> Vec<ToolSpec> {
        self.specs.values().cloned().collect()
    }

    async fn execute(&self, name: &str, arguments: &str) -> Result<ToolResult, AgentError> {
        if name != "get_current_time" {
            return Err(AgentError::InvalidInput(format!("unknown tool: {name}")));
        }

        let args: GetCurrentTimeArgs = if arguments.trim().is_empty() {
            GetCurrentTimeArgs { timezone: None }
        } else {
            serde_json::from_str(arguments).map_err(|e| {
                AgentError::InvalidInput(format!("invalid get_current_time arguments: {e}"))
            })?
        };

        let timezone = args
            .timezone
            .unwrap_or_else(|| "UTC".to_string())
            .parse::<Tz>()
            .map_err(|e| AgentError::InvalidInput(format!("invalid timezone: {e}")))?;

        let now = Utc::now().with_timezone(&timezone);
        Ok(ToolResult {
            call_id: String::new(),
            content: now.to_rfc3339(),
        })
    }
}

#[cfg(test)]
mod tests {
    use agent_domain::ToolRuntime;

    use super::BuiltinToolRuntime;

    #[tokio::test]
    async fn get_current_time_returns_rfc3339() {
        let runtime = BuiltinToolRuntime::new();
        let result = runtime
            .execute("get_current_time", r#"{"timezone":"Asia/Shanghai"}"#)
            .await
            .expect("tool call should succeed");

        assert!(!result.content.is_empty());
        assert!(chrono::DateTime::parse_from_rfc3339(&result.content).is_ok());
    }
}
