use async_trait::async_trait;
use chrono::Utc;
use chrono_tz::Tz;
use serde::Deserialize;
use serde_json::json;

use crate::{Tool, ToolError, ToolInput, ToolOutput, ToolSpec};

pub struct GetCurrentTimeTool;

#[derive(Debug, Deserialize)]
struct GetCurrentTimeArgs {
    timezone: Option<String>,
}

#[async_trait]
impl Tool for GetCurrentTimeTool {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "get_current_time".to_string(),
            description: "Get current time in an IANA timezone".to_string(),
            parameters_schema: json!({
                "type": "object",
                "properties": {
                    "timezone": {
                        "type": "string",
                        "description": "IANA timezone like Asia/Shanghai, defaults to UTC"
                    }
                },
                "required": []
            }),
            strict: false,
            execution_class: crate::ExecutionClass::Local,
        }
    }

    async fn execute(&self, input: ToolInput) -> Result<ToolOutput, ToolError> {
        let args: GetCurrentTimeArgs = if input.arguments_json.trim().is_empty() {
            GetCurrentTimeArgs { timezone: None }
        } else {
            serde_json::from_str(&input.arguments_json)
                .map_err(|e| ToolError::InvalidArguments(e.to_string()))?
        };

        let timezone = args
            .timezone
            .unwrap_or_else(|| "UTC".to_string())
            .parse::<Tz>()
            .map_err(|e| ToolError::InvalidArguments(format!("invalid timezone: {e}")))?;

        let now = Utc::now().with_timezone(&timezone);
        Ok(ToolOutput {
            content_json: json!({ "now": now.to_rfc3339() }).to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use super::*;

    fn input(arguments_json: &str) -> ToolInput {
        ToolInput {
            request_id: "r1".to_string(),
            tool_name: "get_current_time".to_string(),
            arguments_json: arguments_json.to_string(),
            timeout_ms: None,
        }
    }

    #[tokio::test]
    async fn parses_valid_timezone() {
        let output = GetCurrentTimeTool
            .execute(input(r#"{"timezone":"Asia/Shanghai"}"#))
            .await
            .expect("valid timezone should pass");

        let payload: Value = serde_json::from_str(&output.content_json).expect("valid json");
        let now = payload["now"].as_str().expect("now should be string");
        assert!(now.ends_with("+08:00"));
    }

    #[tokio::test]
    async fn defaults_to_utc_when_empty_args() {
        let output = GetCurrentTimeTool
            .execute(input(""))
            .await
            .expect("empty args should default to UTC");

        let payload: Value = serde_json::from_str(&output.content_json).expect("valid json");
        let now = payload["now"].as_str().expect("now should be string");
        assert!(now.ends_with("+00:00") || now.ends_with("Z"));
    }

    #[tokio::test]
    async fn returns_error_for_invalid_timezone() {
        let error = GetCurrentTimeTool
            .execute(input(r#"{"timezone":"Mars/Olympus"}"#))
            .await
            .expect_err("invalid timezone should fail");

        assert!(matches!(error, ToolError::InvalidArguments(_)));
    }
}
