use super::*;
use agent_domain::FinishReason;
use serde_json::json;

#[test]
fn serializes_request_body_correctly() {
    let provider = OpenAiProvider::with_config("k", "https://example.com/v1", "gpt-test");
    let request = LlmRequest {
        messages: vec![agent_domain::ChatMessage {
            role: "user".into(),
            content: "hello".into(),
            tool_calls: None,
            tool_call_id: None,
        }],
        model: None,
        temperature: Some(0.3),
        max_tokens: Some(128),
        tools: vec![agent_domain::ToolSpec {
            name: "get_current_time".to_string(),
            description: "Get current time".to_string(),
            parameters: json!({"type":"object"}),
        }],
    };

    let payload = provider.build_request_body(request);
    let value = serde_json::to_value(payload).expect("serialize payload");

    assert_eq!(value["model"], json!("gpt-test"));
    assert_eq!(
        value["messages"],
        json!([{"role": "user", "content": "hello"}])
    );
    assert_eq!(value["max_tokens"], json!(128));
    assert_eq!(value["tools"][0]["type"], json!("function"));
    assert_eq!(
        value["tools"][0]["function"]["name"],
        json!("get_current_time")
    );

    let temperature = value["temperature"]
        .as_f64()
        .expect("temperature should be a number");
    assert!((temperature - 0.3).abs() < 1e-6);
}

#[test]
fn parses_response_body_correctly() {
    let raw = r#"{
            "model": "gpt-4o-mini",
            "choices": [{"message": {"content": "Hi there"}, "finish_reason": "stop"}],
            "usage": {
                "prompt_tokens": 10,
                "completion_tokens": 5,
                "total_tokens": 15
            }
        }"#;

    let response = OpenAiProvider::parse_success_body(raw).expect("parse success body");

    assert_eq!(response.content, "Hi there");
    assert_eq!(response.model, "gpt-4o-mini");
    assert_eq!(response.finish_reason, FinishReason::Stop);
    assert_eq!(response.tool_calls, vec![]);
    assert_eq!(
        response.usage,
        Some(LlmUsage {
            input_tokens: 10,
            output_tokens: 5,
            total_tokens: 15,
        })
    );
}

#[test]
fn parses_tool_calls_response_body() {
    let raw = r#"{
            "model": "gpt-4o-mini",
            "choices": [{
                "message": {
                    "content": null,
                    "tool_calls": [{
                        "id": "call_1",
                        "type": "function",
                        "function": {
                            "name": "get_current_time",
                            "arguments": "{\"timezone\":\"Asia/Shanghai\"}"
                        }
                    }]
                },
                "finish_reason": "tool_calls"
            }],
            "usage": null
        }"#;

    let response = OpenAiProvider::parse_success_body(raw).expect("parse success body");

    assert_eq!(response.finish_reason, FinishReason::ToolCalls);
    assert_eq!(response.content, "");
    assert_eq!(response.tool_calls.len(), 1);
    assert_eq!(response.tool_calls[0].call_id, "call_1");
    assert_eq!(response.tool_calls[0].name, "get_current_time");
}

#[test]
fn maps_http_errors_correctly() {
    assert_eq!(
        OpenAiProvider::map_http_error(StatusCode::TOO_MANY_REQUESTS, "slow down".into()),
        LlmError::RateLimited
    );

    assert_eq!(
        OpenAiProvider::map_http_error(StatusCode::BAD_REQUEST, "bad payload".into()),
        LlmError::InvalidRequest("bad payload".into())
    );

    assert_eq!(
        OpenAiProvider::map_http_error(StatusCode::INTERNAL_SERVER_ERROR, "oops".into()),
        LlmError::ProviderError("oops".into())
    );
}

#[test]
fn empty_messages_serialization() {
    let provider = OpenAiProvider::with_config("k", "https://example.com/v1", "gpt-test");
    let request = LlmRequest {
        messages: vec![],
        model: None,
        temperature: None,
        max_tokens: None,
        tools: vec![],
    };

    let payload = provider.build_request_body(request);
    let value = serde_json::to_value(payload).expect("serialize payload");

    assert_eq!(value["messages"], json!([]));
    assert_eq!(value["model"], json!("gpt-test"));
    assert!(value.get("tools").is_none());
}

#[test]
fn response_missing_choices_returns_error() {
    let raw = r#"{"model":"gpt-4o-mini","choices":[],"usage":null}"#;

    let err = OpenAiProvider::parse_success_body(raw).expect_err("should fail");

    assert_eq!(err, LlmError::ProviderError("missing choices[0]".into()));
}

#[test]
fn rate_limit_429_maps_to_rate_limited() {
    let err = OpenAiProvider::map_http_error(StatusCode::TOO_MANY_REQUESTS, "retry".into());

    assert_eq!(err, LlmError::RateLimited);
}
