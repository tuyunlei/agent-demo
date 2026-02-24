use std::collections::HashMap;

use agent_domain::{AgentError, ToolResult, ToolRuntime, ToolSpec};
use chrono::Utc;
use chrono_tz::Tz;
use reqwest::Client;
use serde::Deserialize;
use serde_json::json;

const BRAVE_SEARCH_URL: &str = "https://api.search.brave.com/res/v1/web/search";
const DEFAULT_SEARCH_COUNT: u8 = 5;

pub struct BuiltinToolRuntime {
    specs: HashMap<String, ToolSpec>,
    http_client: Client,
    brave_search_api_key: Option<String>,
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
        specs.insert(
            "web_search".to_string(),
            ToolSpec {
                name: "web_search".to_string(),
                description: "Search the web for current information. Use this when the user asks about recent events, facts you're unsure about, or anything that requires up-to-date information.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "query": {
                            "type": "string",
                            "description": "The search query"
                        },
                        "count": {
                            "type": "integer",
                            "description": "Number of results to return (1-10, default 5)"
                        }
                    },
                    "required": ["query"]
                }),
            },
        );

        Self {
            specs,
            http_client: Client::new(),
            brave_search_api_key: std::env::var("BRAVE_SEARCH_API_KEY").ok(),
        }
    }
}

impl Default for BuiltinToolRuntime {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Deserialize)]
struct GetCurrentTimeArgs {
    timezone: Option<String>,
}

#[derive(Deserialize)]
struct WebSearchInput {
    query: String,
    count: Option<u8>,
}

#[derive(Debug)]
struct WebSearchArgs {
    query: String,
    count: u8,
}

#[derive(Deserialize)]
struct BraveSearchResponse {
    web: Option<BraveWebResults>,
}

#[derive(Deserialize)]
struct BraveWebResults {
    results: Vec<BraveWebResult>,
}

#[derive(Deserialize)]
struct BraveWebResult {
    title: String,
    url: String,
    #[serde(default)]
    description: String,
}

fn parse_web_search_args(arguments: &str) -> Result<WebSearchArgs, AgentError> {
    let input: WebSearchInput = serde_json::from_str(arguments)
        .map_err(|e| AgentError::InvalidInput(format!("invalid web_search arguments: {e}")))?;
    let count = input.count.unwrap_or(DEFAULT_SEARCH_COUNT);
    if !(1..=10).contains(&count) {
        return Err(AgentError::InvalidInput(
            "invalid web_search count: must be between 1 and 10".to_string(),
        ));
    }
    Ok(WebSearchArgs {
        query: input.query,
        count,
    })
}

fn format_web_search_results(query: &str, results: &[BraveWebResult]) -> String {
    if results.is_empty() {
        return format!("Search results for \"{query}\":\n\nNo results found.");
    }

    let mut output = format!("Search results for \"{query}\":\n\n");
    for (index, result) in results.iter().enumerate() {
        let item = format!(
            "{}. {}\n   URL: {}\n   {}\n\n",
            index + 1,
            result.title,
            result.url,
            result.description
        );
        output.push_str(&item);
    }
    output.trim_end().to_string()
}

#[async_trait::async_trait]
impl ToolRuntime for BuiltinToolRuntime {
    fn list_tools(&self) -> Vec<ToolSpec> {
        self.specs.values().cloned().collect()
    }

    async fn execute(&self, name: &str, arguments: &str) -> Result<ToolResult, AgentError> {
        match name {
            "get_current_time" => execute_get_current_time(arguments),
            "web_search" => self.execute_web_search(arguments).await,
            _ => Err(AgentError::InvalidInput(format!("unknown tool: {name}"))),
        }
    }
}

fn execute_get_current_time(arguments: &str) -> Result<ToolResult, AgentError> {
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

impl BuiltinToolRuntime {
    async fn execute_web_search(&self, arguments: &str) -> Result<ToolResult, AgentError> {
        let args = parse_web_search_args(arguments)?;
        let Some(api_key) = &self.brave_search_api_key else {
            return Ok(ToolResult {
                call_id: String::new(),
                content:
                    "web_search is unavailable: BRAVE_SEARCH_API_KEY is not set in the environment."
                        .to_string(),
            });
        };

        let response = self
            .http_client
            .get(BRAVE_SEARCH_URL)
            .header("X-Subscription-Token", api_key)
            .query(&[
                ("q", args.query.as_str()),
                ("count", &args.count.to_string()),
            ])
            .send()
            .await;

        let response = match response {
            Ok(response) => response,
            Err(error) => {
                return Ok(ToolResult {
                    call_id: String::new(),
                    content: format!("web_search failed to call Brave Search API: {error}"),
                });
            }
        };

        if !response.status().is_success() {
            return Ok(ToolResult {
                call_id: String::new(),
                content: format!(
                    "web_search failed: Brave Search API returned {}",
                    response.status()
                ),
            });
        }

        let payload = response.json::<BraveSearchResponse>().await;
        let payload = match payload {
            Ok(payload) => payload,
            Err(error) => {
                return Ok(ToolResult {
                    call_id: String::new(),
                    content: format!("web_search failed to parse Brave Search response: {error}"),
                });
            }
        };

        let results = payload.web.map(|w| w.results).unwrap_or_default();
        Ok(ToolResult {
            call_id: String::new(),
            content: format_web_search_results(&args.query, &results),
        })
    }
}

#[cfg(test)]
mod tests {
    use agent_domain::{AgentError, ToolRuntime};

    use super::{BuiltinToolRuntime, parse_web_search_args};

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

    #[tokio::test]
    async fn web_search_returns_helpful_message_when_api_key_missing() {
        let runtime = BuiltinToolRuntime::new();
        let result = runtime
            .execute("web_search", r#"{"query":"rust"}"#)
            .await
            .expect("tool call should not fail when api key missing");

        assert!(result.content.contains("BRAVE_SEARCH_API_KEY"));
    }

    #[test]
    fn web_search_missing_query_returns_invalid_input() {
        let error = parse_web_search_args(r#"{"count":5}"#).expect_err("should fail");
        assert!(matches!(error, AgentError::InvalidInput(_)));
    }

    #[test]
    fn web_search_invalid_count_returns_invalid_input() {
        let error =
            parse_web_search_args(r#"{"query":"rust","count":0}"#).expect_err("should fail");
        assert!(matches!(error, AgentError::InvalidInput(_)));
    }
}
