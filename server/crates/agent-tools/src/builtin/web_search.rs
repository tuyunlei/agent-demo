use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use serde_json::json;

use crate::{Tool, ToolError, ToolInput, ToolOutput, ToolSpec};

const BRAVE_SEARCH_URL: &str = "https://api.search.brave.com/res/v1/web/search";
const DEFAULT_SEARCH_COUNT: u8 = 5;

pub struct WebSearchTool {
    http_client: Client,
    brave_search_api_key: Option<String>,
}

impl WebSearchTool {
    pub fn new(api_key: Option<String>) -> Self {
        Self {
            http_client: Client::new(),
            brave_search_api_key: api_key,
        }
    }

    pub fn from_env() -> Self {
        Self::new(std::env::var("BRAVE_SEARCH_API_KEY").ok())
    }
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

fn parse_web_search_args(arguments: &str) -> Result<WebSearchArgs, ToolError> {
    let input: WebSearchInput = serde_json::from_str(arguments)
        .map_err(|e| ToolError::InvalidArguments(format!("invalid web_search arguments: {e}")))?;

    if input.query.trim().is_empty() {
        return Err(ToolError::InvalidArguments(
            "query cannot be empty".to_string(),
        ));
    }

    let count = input.count.unwrap_or(DEFAULT_SEARCH_COUNT);
    if !(1..=10).contains(&count) {
        return Err(ToolError::InvalidArguments(
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
        return format!("Search results for \"{query}\":\\n\\nNo results found.");
    }

    let mut output = format!("Search results for \"{query}\":\\n\\n");
    for (index, result) in results.iter().enumerate() {
        output.push_str(&format!(
            "{}. {}\\n   URL: {}\\n   {}\\n\\n",
            index + 1,
            result.title,
            result.url,
            result.description
        ));
    }
    output.trim_end().to_string()
}

#[async_trait]
impl Tool for WebSearchTool {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "web_search".to_string(),
            description: "Search the web for current information".to_string(),
            parameters_schema: json!({
                "type": "object",
                "properties": {
                    "query": {"type": "string", "description": "The search query"},
                    "count": {"type": "integer", "description": "Number of results (1-10, default 5)"}
                },
                "required": ["query"]
            }),
            strict: false,
        }
    }

    async fn execute(&self, input: ToolInput) -> Result<ToolOutput, ToolError> {
        let args = parse_web_search_args(&input.arguments_json)?;
        let Some(api_key) = &self.brave_search_api_key else {
            return Ok(ToolOutput {
                content_json: json!({
                    "message": "web_search is unavailable: BRAVE_SEARCH_API_KEY is not set in the environment."
                })
                .to_string(),
            });
        };

        let response = match self
            .http_client
            .get(BRAVE_SEARCH_URL)
            .header("X-Subscription-Token", api_key)
            .query(&[
                ("q", args.query.as_str()),
                ("count", &args.count.to_string()),
            ])
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => {
                return Ok(ToolOutput {
                    content_json: json!({
                        "error": format!("web_search failed to call Brave Search API: {e}")
                    })
                    .to_string(),
                });
            }
        };

        if !response.status().is_success() {
            return Ok(ToolOutput {
                content_json: json!({
                    "error": format!("web_search failed: Brave Search API returned {}", response.status())
                })
                .to_string(),
            });
        }

        let payload = match response.json::<BraveSearchResponse>().await {
            Ok(p) => p,
            Err(e) => {
                return Ok(ToolOutput {
                    content_json: json!({
                        "error": format!("web_search failed to parse Brave Search response: {e}")
                    })
                    .to_string(),
                });
            }
        };

        let results = payload.web.map(|w| w.results).unwrap_or_default();
        Ok(ToolOutput {
            content_json: json!({
                "content": format_web_search_results(&args.query, &results)
            })
            .to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(arguments_json: &str) -> ToolInput {
        ToolInput {
            request_id: "r1".to_string(),
            tool_name: "web_search".to_string(),
            arguments_json: arguments_json.to_string(),
            timeout_ms: None,
        }
    }

    #[test]
    fn parse_web_search_rejects_missing_query() {
        let error = parse_web_search_args(r#"{"count":5}"#).expect_err("query is required");
        assert!(matches!(error, ToolError::InvalidArguments(_)));
    }

    #[test]
    fn parse_web_search_rejects_out_of_range_count() {
        let error = parse_web_search_args(r#"{"query":"rust","count":11}"#)
            .expect_err("count out of range should fail");
        assert!(matches!(error, ToolError::InvalidArguments(_)));
    }

    #[test]
    fn parse_web_search_rejects_empty_query() {
        let error =
            parse_web_search_args(r#"{"query":"   "}"#).expect_err("empty query should fail");
        assert!(matches!(error, ToolError::InvalidArguments(_)));
        assert!(error.to_string().contains("query cannot be empty"));
    }

    #[test]
    fn parse_web_search_defaults_count() {
        let args = parse_web_search_args(r#"{"query":"rust"}"#).expect("valid args");
        assert_eq!(args.count, DEFAULT_SEARCH_COUNT);
    }

    #[test]
    fn tool_spec_exposes_expected_schema() {
        let spec = WebSearchTool::new(Some("key".to_string())).spec();
        assert_eq!(spec.name, "web_search");
        assert_eq!(
            spec.parameters_schema["required"],
            serde_json::json!(["query"])
        );
        assert_eq!(
            spec.parameters_schema["properties"]["count"]["type"],
            "integer"
        );
    }

    #[test]
    fn format_web_search_results_for_empty_and_non_empty_payloads() {
        let empty = format_web_search_results("rust", &[]);
        assert!(empty.contains("No results found"));

        let filled = format_web_search_results(
            "rust",
            &[BraveWebResult {
                title: "Rust Language".to_string(),
                url: "https://www.rust-lang.org".to_string(),
                description: "Official site".to_string(),
            }],
        );
        assert!(filled.contains("1. Rust Language"));
        assert!(filled.contains("URL: https://www.rust-lang.org"));
        assert!(filled.contains("Official site"));
    }

    #[tokio::test]
    async fn execute_without_api_key_returns_friendly_message() {
        let tool = WebSearchTool::new(None);
        let output = tool
            .execute(input(r#"{"query":"rust"}"#))
            .await
            .expect("no api key should still produce output");

        assert!(output.content_json.contains("BRAVE_SEARCH_API_KEY"));
    }
}
