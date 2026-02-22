use super::*;
use serde_json::json;

#[test]
fn serializes_request_body_correctly() {
    let provider = OpenAiProvider::with_config("k", "https://example.com/v1", "gpt-test");
    let request = LlmRequest {
        messages: vec![agent_domain::ChatMessage {
            role: "user".into(),
            content: "hello".into(),
        }],
        model: None,
        temperature: Some(0.3),
        max_tokens: Some(128),
    };

    let payload = provider.build_request_body(request);
    let value = serde_json::to_value(payload).expect("serialize payload");

    assert_eq!(value["model"], json!("gpt-test"));
    assert_eq!(
        value["messages"],
        json!([{"role": "user", "content": "hello"}])
    );
    assert_eq!(value["max_tokens"], json!(128));

    let temperature = value["temperature"]
        .as_f64()
        .expect("temperature should be a number");
    assert!((temperature - 0.3).abs() < 1e-6);
}

#[test]
fn parses_response_body_correctly() {
    let raw = r#"{
            "model": "gpt-4o-mini",
            "choices": [{"message": {"content": "Hi there"}}],
            "usage": {
                "prompt_tokens": 10,
                "completion_tokens": 5,
                "total_tokens": 15
            }
        }"#;

    let response = OpenAiProvider::parse_success_body(raw).expect("parse success body");

    assert_eq!(response.content, "Hi there");
    assert_eq!(response.model, "gpt-4o-mini");
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
    };

    let payload = provider.build_request_body(request);
    let value = serde_json::to_value(payload).expect("serialize payload");

    assert_eq!(value["messages"], json!([]));
    assert_eq!(value["model"], json!("gpt-test"));
}

#[test]
fn response_missing_choices_returns_error() {
    let raw = r#"{"model":"gpt-4o-mini","choices":[],"usage":null}"#;

    let err = OpenAiProvider::parse_success_body(raw).expect_err("should fail");

    assert_eq!(
        err,
        LlmError::ProviderError("missing choices[0].message.content".into())
    );
}

#[test]
fn rate_limit_429_maps_to_rate_limited() {
    let err = OpenAiProvider::map_http_error(StatusCode::TOO_MANY_REQUESTS, "retry".into());

    assert_eq!(err, LlmError::RateLimited);
}
