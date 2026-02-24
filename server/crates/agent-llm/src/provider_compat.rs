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
