use std::sync::{Arc, Mutex};

use agent_domain::{MessageStore, StoreError, StoredMessage};
use agent_proto::session_service_server::SessionService;
use agent_proto::{ListSessionMessagesRequest, PaginationRequest};

use super::*;

#[derive(Default)]
struct MockMessageStore {
    captured: Arc<Mutex<Vec<(String, i64)>>>,
}

#[async_trait::async_trait]
impl MessageStore for MockMessageStore {
    async fn create_session(&self, _user_id: &str, _agent_id: &str) -> Result<String, StoreError> {
        unreachable!("not used in test")
    }

    async fn save_message(
        &self,
        _session_id: &str,
        _role: &str,
        _content: &str,
    ) -> Result<String, StoreError> {
        unreachable!("not used in test")
    }

    async fn get_session_messages(
        &self,
        session_id: &str,
        limit: i64,
    ) -> Result<Vec<StoredMessage>, StoreError> {
        self.captured
            .lock()
            .expect("lock captured")
            .push((session_id.to_string(), limit));

        Ok(vec![StoredMessage {
            id: "msg-001".to_string(),
            session_id: session_id.to_string(),
            role: "assistant".to_string(),
            content: "hello from history".to_string(),
            created_at: 1700000000,
        }])
    }

    async fn get_or_create_default_session(&self, _user_id: &str) -> Result<String, StoreError> {
        unreachable!("not used in test")
    }
}

#[tokio::test]
async fn list_messages_returns_stored_messages() {
    let store = Arc::new(MockMessageStore::default());
    let captured = store.captured.clone();
    let handler = SessionServiceHandler::new(store);

    let mut request = tonic::Request::new(ListSessionMessagesRequest {
        session_id: "session-001".to_string(),
        pagination: Some(PaginationRequest {
            page_size: 120,
            page_token: String::new(),
        }),
        created_at_order: 0,
    });
    request
        .extensions_mut()
        .insert(UserId("user-001".to_string()));

    let response = handler.list_session_messages(request).await.unwrap();
    let payload = response.into_inner();

    assert_eq!(payload.messages.len(), 1);
    let msg = &payload.messages[0];
    assert_eq!(msg.message_id, "msg-001");
    assert_eq!(msg.session_id, "session-001");
    assert_eq!(msg.role, "assistant");
    assert_eq!(msg.blocks.len(), 1);

    let calls = captured.lock().expect("lock captured");
    assert_eq!(calls.as_slice(), &[("session-001".to_string(), 120)]);
}

#[tokio::test]
async fn list_messages_empty_session_id_returns_error() {
    let store = Arc::new(MockMessageStore::default());
    let handler = SessionServiceHandler::new(store);

    let mut request = tonic::Request::new(ListSessionMessagesRequest {
        session_id: "   ".to_string(),
        pagination: None,
        created_at_order: 0,
    });
    request
        .extensions_mut()
        .insert(UserId("user-001".to_string()));

    let err = handler.list_session_messages(request).await.unwrap_err();
    assert_eq!(err.code(), tonic::Code::InvalidArgument);
}
