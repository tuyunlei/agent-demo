use reqwest::StatusCode;
use serde_json::json;

use super::*;
use crate::types::{
    FinishReason, LlmRequest, LlmRequestConfig, LlmRequestMetadata, LlmUsage, ModelMessage,
    ToolSpec,
};

#[test]
fn serializes_request_body_correctly() {
    let provider = OpenAiProvider::with_config("k", "https://example.com/v1", "gpt-test");
    let request = LlmRequest {
        messages: vec![ModelMessage {
            role: "user".into(),
            content: "hello".into(),
            tool_calls: None,
            tool_call_id: None,
        }],
        tool_specs: vec![ToolSpec {
            name: "get_current_time".to_string(),
            description: "Get current time".to_string(),
            parameters_schema: json!({"type":"object"}),
            strict: true,
        }],
        config: LlmRequestConfig {
            model: String::new(),
            temperature: Some(0.3),
            top_p: Some(0.9),
            max_tokens: Some(128),
            timeout: None,
            json_mode: true,
        },
        metadata: LlmRequestMetadata::default(),
    };

    let payload = provider.build_request_body(request);
    let value = serde_json::to_value(payload).expect("serialize payload");

    assert_eq!(value["model"], json!("gpt-test"));
    assert_eq!(
        value["messages"],
        json!([{"role": "user", "content": "hello"}])
    );
    assert_eq!(value["max_tokens"], json!(128));
    let top_p = value["top_p"].as_f64().expect("top_p should be number");
    assert!((top_p - 0.9).abs() < 1e-6);
    assert_eq!(value["tools"][0]["type"], json!("function"));
    assert_eq!(
        value["tools"][0]["function"]["name"],
        json!("get_current_time")
    );
    assert_eq!(value["response_format"]["type"], json!("json_object"));
}

#[test]
fn parses_response_body_correctly() {
    let raw = r#"{
            "id": "resp_1",
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
    assert_eq!(response.provider, "openai-compatible");
    assert_eq!(response.response_id, Some("resp_1".to_string()));
    assert_eq!(response.finish_reason, FinishReason::Stop);
    assert_eq!(response.tool_calls, vec![]);
    assert_eq!(
        response.usage,
        Some(LlmUsage {
            prompt_tokens: 10,
            completion_tokens: 5,
            total_tokens: 15,
            cache_read_tokens: None,
            cache_write_tokens: None,
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
                        "function": {"name": "get_current_time", "arguments": "{\"timezone\":\"Asia/Shanghai\"}"}
                    }]
                },
                "finish_reason": "tool_calls"
            }],
            "usage": null
        }"#;

    let response = OpenAiProvider::parse_success_body(raw).expect("parse success body");
    assert_eq!(response.finish_reason, FinishReason::ToolCalls);
    assert_eq!(response.content, "");
    assert_eq!(response.tool_calls[0].call_id, "call_1");
}

#[test]
fn maps_http_errors_correctly() {
    assert_eq!(
        OpenAiProvider::map_http_error(StatusCode::TOO_MANY_REQUESTS, "slow down".into()),
        LlmError::RateLimit
    );
    assert_eq!(
        OpenAiProvider::map_http_error(StatusCode::UNAUTHORIZED, "bad key".into()),
        LlmError::AuthError
    );
    assert_eq!(
        OpenAiProvider::map_http_error(StatusCode::BAD_REQUEST, "bad payload".into()),
        LlmError::InvalidRequest("bad payload".into())
    );
}

#[test]
fn response_missing_choices_returns_error() {
    let raw = r#"{"model":"gpt-4o-mini","choices":[],"usage":null}"#;
    let err = OpenAiProvider::parse_success_body(raw).expect_err("should fail");
    assert_eq!(err, LlmError::InvalidResponse("missing choices[0]".into()));
}
