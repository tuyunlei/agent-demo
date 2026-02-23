use std::sync::{Arc, Mutex};

use agent_domain::{MessageStore, StoreError, StoredMessage, StoredSession};
use agent_proto::session_service_server::SessionService;
use agent_proto::{
    CreateSessionRequest, GetSessionRequest, ListSessionMessagesRequest, ListSessionsRequest,
    PaginationRequest,
};

use super::*;

#[derive(Default)]
struct MockMessageStore {
    captured_messages: Arc<Mutex<Vec<(String, i64)>>>,
}

#[async_trait::async_trait]
impl MessageStore for MockMessageStore {
    async fn create_session(&self, _user_id: &str, _agent_id: &str) -> Result<String, StoreError> {
        unreachable!("not used in test")
    }

    async fn create_session_with_title(
        &self,
        user_id: &str,
        title: &str,
    ) -> Result<StoredSession, StoreError> {
        Ok(sample_session("session-new", user_id, title))
    }

    async fn list_sessions(&self, user_id: &str) -> Result<Vec<StoredSession>, StoreError> {
        Ok(vec![sample_session("session-001", user_id, "First")])
    }

    async fn get_session(
        &self,
        user_id: &str,
        session_id: &str,
    ) -> Result<Option<StoredSession>, StoreError> {
        if session_id == "missing" {
            Ok(None)
        } else {
            Ok(Some(sample_session(session_id, user_id, "Found")))
        }
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
        self.captured_messages
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
async fn create_session_returns_session() {
    let handler = SessionServiceHandler::new(Arc::new(MockMessageStore::default()));
    let mut request = tonic::Request::new(CreateSessionRequest {
        title: "My Session".to_string(),
    });
    request
        .extensions_mut()
        .insert(UserId("user-001".to_string()));

    let response = handler.create_session(request).await.unwrap().into_inner();
    let session = response.session.unwrap();

    assert_eq!(session.user_id, "user-001");
    assert_eq!(session.title, "My Session");
}

#[tokio::test]
async fn get_session_empty_session_id_returns_error() {
    let handler = SessionServiceHandler::new(Arc::new(MockMessageStore::default()));
    let mut request = tonic::Request::new(GetSessionRequest {
        session_id: "   ".to_string(),
    });
    request
        .extensions_mut()
        .insert(UserId("user-001".to_string()));

    let err = handler.get_session(request).await.unwrap_err();
    assert_eq!(err.code(), tonic::Code::InvalidArgument);
}

#[tokio::test]
async fn get_session_not_found_returns_error() {
    let handler = SessionServiceHandler::new(Arc::new(MockMessageStore::default()));
    let mut request = tonic::Request::new(GetSessionRequest {
        session_id: "missing".to_string(),
    });
    request
        .extensions_mut()
        .insert(UserId("user-001".to_string()));

    let err = handler.get_session(request).await.unwrap_err();
    assert_eq!(err.code(), tonic::Code::NotFound);
}

#[tokio::test]
async fn list_sessions_returns_items() {
    let handler = SessionServiceHandler::new(Arc::new(MockMessageStore::default()));
    let mut request = tonic::Request::new(ListSessionsRequest::default());
    request
        .extensions_mut()
        .insert(UserId("user-001".to_string()));

    let payload = handler.list_sessions(request).await.unwrap().into_inner();

    assert_eq!(payload.sessions.len(), 1);
    assert_eq!(payload.sessions[0].session_id, "session-001");
}

#[tokio::test]
async fn list_messages_returns_stored_messages() {
    let store = Arc::new(MockMessageStore::default());
    let captured = store.captured_messages.clone();
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
    assert_eq!(payload.messages[0].message_id, "msg-001");

    let calls = captured.lock().expect("lock captured");
    assert_eq!(calls.as_slice(), &[("session-001".to_string(), 120)]);
}

fn sample_session(id: &str, user_id: &str, title: &str) -> StoredSession {
    StoredSession {
        id: id.to_string(),
        user_id: user_id.to_string(),
        agent_id: "".to_string(),
        title: title.to_string(),
        summary: String::new(),
        created_at: 1700000000,
        updated_at: 1700000001,
        last_message_at: Some(1700000001),
        archived: false,
    }
}
