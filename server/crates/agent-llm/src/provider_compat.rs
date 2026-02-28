use agent_domain as domain;

use crate::error::LlmError;
use crate::types::{
    FinishReason, LlmRequest, LlmRequestConfig, LlmRequestMetadata, LlmResponse, ToolCall, ToolSpec,
};

pub fn map_from_domain_request(request: domain::LlmRequest, default_model: &str) -> LlmRequest {
    LlmRequest {
        messages: request
            .messages
            .into_iter()
            .map(|m| crate::types::ModelMessage {
                role: m.role,
                content: m.content,
                tool_calls: m.tool_calls.map(|calls| {
                    calls
                        .into_iter()
                        .map(|c| ToolCall {
                            call_id: c.call_id,
                            name: c.name,
                            arguments: c.arguments,
                        })
                        .collect()
                }),
                tool_call_id: m.tool_call_id,
            })
            .collect(),
        tool_specs: request
            .tools
            .into_iter()
            .map(|t| ToolSpec {
                name: t.name,
                description: t.description,
                parameters_schema: t.parameters,
                strict: false,
            })
            .collect(),
        builtin_tools: vec![],
        previous_response_id: None,
        config: LlmRequestConfig {
            model: request.model.unwrap_or_else(|| default_model.to_string()),
            temperature: request.temperature,
            top_p: None,
            max_tokens: request.max_tokens,
            timeout: None,
            json_mode: false,
        },
        metadata: LlmRequestMetadata::default(),
    }
}

pub fn map_to_domain_response(response: LlmResponse) -> domain::LlmResponse {
    domain::LlmResponse {
        content: response.content,
        model: response.model,
        usage: response.usage.map(|u| domain::LlmUsage {
            input_tokens: u.prompt_tokens,
            output_tokens: u.completion_tokens,
            total_tokens: u.total_tokens,
        }),
        tool_calls: response
            .tool_calls
            .into_iter()
            .map(|c| domain::ToolCall {
                call_id: c.call_id,
                name: c.name,
                arguments: c.arguments,
            })
            .collect(),
        finish_reason: match response.finish_reason {
            FinishReason::ToolCalls => domain::FinishReason::ToolCalls,
            FinishReason::Length => domain::FinishReason::Length,
            _ => domain::FinishReason::Stop,
        },
    }
}

pub fn map_to_domain_error(error: LlmError) -> domain::LlmError {
    match error {
        LlmError::RateLimit => domain::LlmError::RateLimited,
        LlmError::Timeout => domain::LlmError::Timeout,
        LlmError::InvalidRequest(msg) => domain::LlmError::InvalidRequest(msg),
        other => domain::LlmError::ProviderError(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn map_from_domain_request_performs_full_mapping() {
        let request = domain::LlmRequest {
            messages: vec![domain::ChatMessage {
                role: "assistant".into(),
                content: "content".into(),
                tool_calls: Some(vec![domain::ToolCall {
                    call_id: "call_1".into(),
                    name: "tool_a".into(),
                    arguments: "{\"x\":1}".into(),
                }]),
                tool_call_id: Some("call_1".into()),
            }],
            model: None,
            temperature: Some(0.2),
            max_tokens: Some(64),
            tools: vec![domain::ToolSpec {
                name: "tool_a".into(),
                description: "desc".into(),
                parameters: json!({"type": "object", "properties": {"x": {"type": "number"}}}),
            }],
        };

        let mapped = map_from_domain_request(request, "fallback-model");

        assert_eq!(mapped.messages.len(), 1);
        assert_eq!(mapped.messages[0].role, "assistant");
        assert_eq!(mapped.messages[0].content, "content");
        assert_eq!(mapped.messages[0].tool_call_id, Some("call_1".into()));
        assert_eq!(
            mapped.messages[0].tool_calls.as_ref().map(Vec::len),
            Some(1)
        );
        assert_eq!(
            mapped.messages[0].tool_calls.as_ref().expect("tool calls")[0].call_id,
            "call_1"
        );

        assert_eq!(mapped.tool_specs.len(), 1);
        assert_eq!(mapped.tool_specs[0].name, "tool_a");
        assert_eq!(mapped.tool_specs[0].description, "desc");
        assert_eq!(
            mapped.tool_specs[0].parameters_schema,
            json!({"type": "object", "properties": {"x": {"type": "number"}}})
        );
        assert!(!mapped.tool_specs[0].strict);

        assert_eq!(mapped.config.model, "fallback-model");
        assert_eq!(mapped.config.temperature, Some(0.2));
        assert_eq!(mapped.config.max_tokens, Some(64));
        assert_eq!(mapped.config.top_p, None);
        assert_eq!(mapped.config.timeout, None);
        assert!(!mapped.config.json_mode);
        assert_eq!(mapped.metadata, LlmRequestMetadata::default());

        let explicit = map_from_domain_request(
            domain::LlmRequest {
                model: Some("explicit-model".into()),
                messages: vec![],
                temperature: None,
                max_tokens: None,
                tools: vec![],
            },
            "fallback-model",
        );
        assert_eq!(explicit.config.model, "explicit-model");
    }

    #[test]
    fn map_to_domain_response_maps_tool_calls_and_finish_reason() {
        let response = LlmResponse {
            content: "answer".into(),
            tool_calls: vec![ToolCall {
                call_id: "c1".into(),
                name: "tool_a".into(),
                arguments: "{}".into(),
            }],
            finish_reason: FinishReason::ToolCalls,
            usage: Some(crate::types::LlmUsage {
                prompt_tokens: 10,
                completion_tokens: 3,
                total_tokens: 13,
                cache_read_tokens: None,
                cache_write_tokens: None,
            }),
            model: "m1".into(),
            provider: "p1".into(),
            response_id: Some("rid".into()),
        };

        let mapped = map_to_domain_response(response);

        assert_eq!(mapped.content, "answer");
        assert_eq!(mapped.model, "m1");
        assert_eq!(mapped.finish_reason, domain::FinishReason::ToolCalls);
        assert_eq!(mapped.tool_calls.len(), 1);
        assert_eq!(
            mapped.tool_calls[0],
            domain::ToolCall {
                call_id: "c1".into(),
                name: "tool_a".into(),
                arguments: "{}".into(),
            }
        );
        assert_eq!(
            mapped.usage,
            Some(domain::LlmUsage {
                input_tokens: 10,
                output_tokens: 3,
                total_tokens: 13,
            })
        );

        let length = map_to_domain_response(LlmResponse {
            finish_reason: FinishReason::Length,
            usage: None,
            tool_calls: vec![],
            content: String::new(),
            model: "m".into(),
            provider: "p".into(),
            response_id: None,
        });
        assert_eq!(length.finish_reason, domain::FinishReason::Length);

        let stop = map_to_domain_response(LlmResponse {
            finish_reason: FinishReason::Error,
            usage: None,
            tool_calls: vec![],
            content: String::new(),
            model: "m".into(),
            provider: "p".into(),
            response_id: None,
        });
        assert_eq!(stop.finish_reason, domain::FinishReason::Stop);
    }

    #[test]
    fn map_to_domain_error_maps_each_llm_error_variant() {
        assert_eq!(
            map_to_domain_error(LlmError::RateLimit),
            domain::LlmError::RateLimited
        );
        assert_eq!(
            map_to_domain_error(LlmError::Timeout),
            domain::LlmError::Timeout
        );
        assert_eq!(
            map_to_domain_error(LlmError::InvalidRequest("bad".into())),
            domain::LlmError::InvalidRequest("bad".into())
        );

        let provider_cases = [
            (
                LlmError::AuthError,
                domain::LlmError::ProviderError("authentication failed".into()),
            ),
            (
                LlmError::ProviderDown,
                domain::LlmError::ProviderError("provider unavailable".into()),
            ),
            (
                LlmError::InvalidResponse("broken".into()),
                domain::LlmError::ProviderError("invalid response: broken".into()),
            ),
            (
                LlmError::UnsupportedCapability {
                    provider: "p".into(),
                    capability: "stream".into(),
                },
                domain::LlmError::ProviderError(
                    "unsupported capability: provider=p capability=stream".into(),
                ),
            ),
            (
                LlmError::Transport("io".into()),
                domain::LlmError::ProviderError("transport error: io".into()),
            ),
            (
                LlmError::Internal("bug".into()),
                domain::LlmError::ProviderError("internal: bug".into()),
            ),
        ];

        for (input, expected) in provider_cases {
            assert_eq!(map_to_domain_error(input), expected);
        }
    }
}
