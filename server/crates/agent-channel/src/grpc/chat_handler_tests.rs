use std::sync::Arc;

use agent_orchestrator::{ChatRuntime, TurnError, TurnFinishReason, TurnInput, TurnOutput};
use agent_proto::{ContentBlock, TextBlock, content_block};

use super::*;

struct MockChatRuntime {
    result: Result<TurnOutput, TurnError>,
}

#[async_trait::async_trait]
impl ChatRuntime for MockChatRuntime {
    async fn run_turn(&self, _input: TurnInput) -> Result<TurnOutput, TurnError> {
        self.result.clone()
    }
}

fn text_block(text: &str) -> ContentBlock {
    ContentBlock {
        kind: Some(content_block::Kind::Text(TextBlock {
            text: text.to_string(),
        })),
    }
}

#[tokio::test]
async fn test_send_message_calls_runtime_chain() {
    let runtime = Arc::new(MockChatRuntime {
        result: Ok(TurnOutput {
            session_id: "session-unknown".to_string(),
            assistant_text: "mocked-reply".to_string(),
            finish_reason: TurnFinishReason::Stop,
            tool_iterations: 0,
        }),
    });
    let handler = ChatServiceHandler::new(runtime);

    let request = tonic::Request::new(SendMessageRequest {
        request_id: "test-123".to_string(),
        session_id: "".to_string(),
        agent_id: "".to_string(),
        content: vec![text_block("hello runtime")],
        metadata: Default::default(),
    });

    let response = handler.send_message(request).await.unwrap();
    let resp = response.into_inner();

    assert_eq!(resp.request_id, "test-123");
    assert_eq!(resp.session_id, "session-unknown");
    assert_eq!(resp.assistant_content.len(), 1);
}

#[tokio::test]
async fn test_send_message_requires_text_content() {
    let runtime = Arc::new(MockChatRuntime {
        result: Err(TurnError::InvalidInput("should not be called".to_string())),
    });
    let handler = ChatServiceHandler::new(runtime);

    let request = tonic::Request::new(SendMessageRequest {
        request_id: "test-123".to_string(),
        session_id: "".to_string(),
        agent_id: "".to_string(),
        content: vec![],
        metadata: Default::default(),
    });

    let err = handler.send_message(request).await.unwrap_err();
    assert_eq!(err.code(), tonic::Code::InvalidArgument);
}
