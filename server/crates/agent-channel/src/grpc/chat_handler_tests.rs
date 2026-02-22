use std::sync::{Arc, Mutex};

use agent_domain::{
    LlmProvider, LlmRequest, LlmResponse, LlmUsage, MessageStore, StoreError, StoredMessage,
};
use agent_proto::{ContentBlock, TextBlock};

use super::*;

struct MockLlmProvider {
    captured: Arc<Mutex<Vec<LlmRequest>>>,
}

#[async_trait::async_trait]
impl LlmProvider for MockLlmProvider {
    async fn generate(&self, request: LlmRequest) -> Result<LlmResponse, LlmError> {
        self.captured.lock().expect("lock captured").push(request);
        Ok(LlmResponse {
            content: "mocked-reply".to_string(),
            model: "mock-model".to_string(),
            usage: Some(LlmUsage {
                input_tokens: 10,
                output_tokens: 10,
                total_tokens: 20,
            }),
        })
    }
}

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
    let captured = Arc::new(Mutex::new(Vec::new()));
    let provider = Arc::new(MockLlmProvider {
        captured: captured.clone(),
    });
    let runtime = Arc::new(AgentRuntime::new(provider, Arc::new(MockMessageStore)));
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

    let requests = captured.lock().expect("lock captured");
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].messages[0].role, "system");
    assert_eq!(requests[0].messages[1].content, "hello runtime");
}

#[tokio::test]
async fn test_send_message_requires_text_content() {
    let captured = Arc::new(Mutex::new(Vec::new()));
    let provider = Arc::new(MockLlmProvider {
        captured: captured.clone(),
    });
    let runtime = Arc::new(AgentRuntime::new(provider, Arc::new(MockMessageStore)));
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
