use reqwest::StatusCode;
use serde_json::json;

use super::*;
use crate::provider_wire::{OpenAiFunctionCall, OpenAiToolCall};
use crate::types::{
    FinishReason, LlmRequest, LlmRequestConfig, LlmRequestMetadata, LlmUsage, ModelMessage,
    ToolCall, ToolSpec,
};

#[test]
fn build_request_body_uses_default_model_and_serializes_tools_json_mode() {
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
        builtin_tools: vec![],
        previous_response_id: None,
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
fn build_request_body_preserves_explicit_model_and_message_tool_calls() {
    let provider = OpenAiProvider::with_config("k", "https://example.com/v1", "gpt-test");
    let request = LlmRequest {
        messages: vec![ModelMessage {
            role: "assistant".into(),
            content: String::new(),
            tool_calls: Some(vec![ToolCall {
                call_id: "call_1".into(),
                name: "tool_name".into(),
                arguments: "{\"x\":1}".into(),
            }]),
            tool_call_id: Some("tool_call_1".into()),
        }],
        tool_specs: vec![],
        builtin_tools: vec![],
        previous_response_id: None,
        config: LlmRequestConfig {
            model: "gpt-4o".into(),
            temperature: None,
            top_p: None,
            max_tokens: None,
            timeout: None,
            json_mode: false,
        },
        metadata: LlmRequestMetadata::default(),
    };

    let payload = provider.build_request_body(request);
    let value = serde_json::to_value(payload).expect("serialize payload");

    assert_eq!(value["model"], json!("gpt-4o"));
    assert_eq!(
        value["messages"][0]["tool_calls"][0]["type"],
        json!("function")
    );
    assert_eq!(
        value["messages"][0]["tool_calls"][0]["function"]["name"],
        json!("tool_name")
    );
    assert_eq!(value["messages"][0]["tool_call_id"], json!("tool_call_1"));
    assert!(value.get("temperature").is_none());
    assert!(value.get("top_p").is_none());
    assert!(value.get("max_tokens").is_none());
    assert!(value.get("tools").is_none());
    assert!(value.get("response_format").is_none());
}

#[test]
fn build_request_body_without_tools_keeps_temperature() {
    let provider = OpenAiProvider::with_config("k", "https://example.com/v1", "gpt-test");
    let request = LlmRequest {
        messages: vec![ModelMessage {
            role: "user".into(),
            content: "ping".into(),
            tool_calls: None,
            tool_call_id: None,
        }],
        tool_specs: vec![],
        builtin_tools: vec![],
        previous_response_id: None,
        config: LlmRequestConfig {
            model: String::new(),
            temperature: Some(0.7),
            top_p: None,
            max_tokens: None,
            timeout: None,
            json_mode: false,
        },
        metadata: LlmRequestMetadata::default(),
    };

    let payload = provider.build_request_body(request);
    let value = serde_json::to_value(payload).expect("serialize payload");

    assert_eq!(value["model"], json!("gpt-test"));
    let temperature = value["temperature"]
        .as_f64()
        .expect("temperature should be number");
    assert!((temperature - 0.7).abs() < 1e-6);
    assert!(value.get("tools").is_none());
}

#[test]
fn build_request_body_with_tools_allows_none_temperature() {
    let provider = OpenAiProvider::with_config("k", "https://example.com/v1", "gpt-test");
    let request = LlmRequest {
        messages: vec![ModelMessage {
            role: "user".into(),
            content: "use tool".into(),
            tool_calls: None,
            tool_call_id: None,
        }],
        tool_specs: vec![ToolSpec {
            name: "tool_a".to_string(),
            description: "desc".to_string(),
            parameters_schema: json!({"type":"object"}),
            strict: false,
        }],
        builtin_tools: vec![],
        previous_response_id: None,
        config: LlmRequestConfig {
            model: "gpt-4o".into(),
            temperature: None,
            top_p: None,
            max_tokens: Some(16),
            timeout: None,
            json_mode: false,
        },
        metadata: LlmRequestMetadata::default(),
    };

    let payload = provider.build_request_body(request);
    let value = serde_json::to_value(payload).expect("serialize payload");

    assert_eq!(value["model"], json!("gpt-4o"));
    assert!(value.get("temperature").is_none());
    assert_eq!(value["tools"][0]["function"]["name"], json!("tool_a"));
}

#[test]
fn parse_success_body_parses_normal_response() {
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
fn parse_success_body_parses_tool_calls_response() {
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
    assert_eq!(response.tool_calls.len(), 1);
    assert_eq!(response.tool_calls[0].call_id, "call_1");
    assert_eq!(response.tool_calls[0].name, "get_current_time");
}

#[test]
fn parse_success_body_returns_error_when_choices_missing() {
    let raw = r#"{"model":"gpt-4o-mini","choices":[],"usage":null}"#;
    let err = OpenAiProvider::parse_success_body(raw).expect_err("should fail");
    assert_eq!(err, LlmError::InvalidResponse("missing choices[0]".into()));
}

#[test]
fn map_http_error_covers_expected_statuses() {
    assert_eq!(
        OpenAiProvider::map_http_error(StatusCode::TOO_MANY_REQUESTS, "slow down".into()),
        LlmError::RateLimit
    );
    assert_eq!(
        OpenAiProvider::map_http_error(StatusCode::UNAUTHORIZED, "bad key".into()),
        LlmError::AuthError
    );
    assert_eq!(
        OpenAiProvider::map_http_error(StatusCode::INTERNAL_SERVER_ERROR, "oops".into()),
        LlmError::ProviderDown
    );
    assert_eq!(
        OpenAiProvider::map_http_error(StatusCode::BAD_REQUEST, "bad payload".into()),
        LlmError::InvalidRequest("bad payload".into())
    );
    let err = OpenAiProvider::map_http_error(StatusCode::FOUND, "redirect".into());
    assert_eq!(
        err,
        LlmError::Transport("unexpected status 302 Found: redirect".into())
    );
}

#[test]
fn parse_tool_calls_handles_none_empty_and_multiple() {
    assert_eq!(parse_tool_calls(None), Vec::<ToolCall>::new());
    assert_eq!(parse_tool_calls(Some(vec![])), Vec::<ToolCall>::new());

    let calls = parse_tool_calls(Some(vec![
        OpenAiToolCall {
            id: "call_1".into(),
            call_type: "function".into(),
            function: OpenAiFunctionCall {
                name: "tool_a".into(),
                arguments: "{\"a\":1}".into(),
            },
        },
        OpenAiToolCall {
            id: "call_2".into(),
            call_type: "function".into(),
            function: OpenAiFunctionCall {
                name: "tool_b".into(),
                arguments: "{\"b\":2}".into(),
            },
        },
    ]));

    assert_eq!(calls.len(), 2);
    assert_eq!(calls[0].call_id, "call_1");
    assert_eq!(calls[1].name, "tool_b");
}

#[test]
fn parse_finish_reason_handles_known_and_unknown_values() {
    assert_eq!(parse_finish_reason(Some("stop".into())), FinishReason::Stop);
    assert_eq!(
        parse_finish_reason(Some("tool_calls".into())),
        FinishReason::ToolCalls
    );
    assert_eq!(
        parse_finish_reason(Some("length".into())),
        FinishReason::Length
    );
    assert_eq!(
        parse_finish_reason(Some("something-else".into())),
        FinishReason::Stop
    );
    assert_eq!(parse_finish_reason(None), FinishReason::Stop);
}
