use std::sync::Arc;

use agent_domain::{MessageStore, StoreError, StoredMessage};
use agent_llm::mock::MockLlmProvider;
use agent_memory::NoopCompactionService;
use agent_orchestrator::{TurnExecutor, TurnExecutorConfig};
use agent_proto::{ContentBlock, TextBlock};
use agent_tools::DefaultToolRuntime;

use super::*;

#[derive(Default)]
struct MockMessageStore;

#[async_trait::async_trait]
impl MessageStore for MockMessageStore {
    async fn create_session(&self, user_id: &str, _agent_id: &str) -> Result<String, StoreError> {
        Ok(format!("session-{user_id}"))
    }

    async fn save_message(
        &self,
        _session_id: &str,
        _role: &str,
        _content: &str,
    ) -> Result<String, StoreError> {
        Ok("msg-id".to_string())
    }

    async fn get_session_messages(
        &self,
        session_id: &str,
        _limit: i64,
    ) -> Result<Vec<StoredMessage>, StoreError> {
        Ok(vec![StoredMessage {
            id: "m1".to_string(),
            session_id: session_id.to_string(),
            role: "user".to_string(),
            content: "hello runtime".to_string(),
            created_at: 1,
        }])
    }

    async fn get_or_create_default_session(&self, user_id: &str) -> Result<String, StoreError> {
        Ok(format!("session-{user_id}"))
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
    let runtime = Arc::new(TurnExecutor::new(
        Arc::new(MockLlmProvider::with_text("mocked-reply")),
        Arc::new(DefaultToolRuntime::new()),
        Arc::new(MockMessageStore),
        Arc::new(NoopCompactionService::new()),
        TurnExecutorConfig::default(),
    ));
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
    let runtime = Arc::new(TurnExecutor::new(
        Arc::new(MockLlmProvider::with_text("mocked-reply")),
        Arc::new(DefaultToolRuntime::new()),
        Arc::new(MockMessageStore),
        Arc::new(NoopCompactionService::new()),
        TurnExecutorConfig::default(),
    ));
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
