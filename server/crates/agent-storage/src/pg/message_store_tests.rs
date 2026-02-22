use std::sync::{Arc, Mutex};

use agent_domain::MessageStore;

use super::*;

#[derive(Default)]
struct InMemoryMessageMock {
    sessions: Arc<Mutex<Vec<String>>>,
    messages: Arc<Mutex<Vec<StoredMessage>>>,
}

#[async_trait::async_trait]
impl MessageStore for InMemoryMessageMock {
    async fn create_session(&self, user_id: &str, _agent_id: &str) -> Result<String, StoreError> {
        let id = format!("session-{user_id}");
        self.sessions
            .lock()
            .expect("lock sessions")
            .push(id.clone());
        Ok(id)
    }

    async fn save_message(
        &self,
        session_id: &str,
        role: &str,
        content: &str,
    ) -> Result<String, StoreError> {
        let id = format!("msg-{}", self.messages.lock().expect("lock").len() + 1);
        self.messages
            .lock()
            .expect("lock messages")
            .push(StoredMessage {
                id: id.clone(),
                session_id: session_id.to_string(),
                role: role.to_string(),
                content: content.to_string(),
                created_at: 1,
            });
        Ok(id)
    }

    async fn get_session_messages(
        &self,
        session_id: &str,
        _limit: i64,
    ) -> Result<Vec<StoredMessage>, StoreError> {
        let items = self
            .messages
            .lock()
            .expect("lock messages")
            .iter()
            .filter(|item| item.session_id == session_id)
            .cloned()
            .collect();
        Ok(items)
    }

    async fn get_or_create_default_session(&self, user_id: &str) -> Result<String, StoreError> {
        let first = self
            .sessions
            .lock()
            .expect("lock sessions")
            .first()
            .cloned();
        match first {
            Some(id) => Ok(id),
            None => self.create_session(user_id, "").await,
        }
    }
}

#[tokio::test]
async fn save_message_mock_works() {
    let store = InMemoryMessageMock::default();
    let session_id = store.create_session("user-1", "").await.expect("session");

    let id = store
        .save_message(&session_id, "user", "hello")
        .await
        .expect("saved");

    assert_eq!(id, "msg-1");
}

#[tokio::test]
async fn get_messages_mock_works() {
    let store = InMemoryMessageMock::default();
    let session_id = store.create_session("user-2", "").await.expect("session");
    store
        .save_message(&session_id, "user", "hello")
        .await
        .expect("saved");

    let messages = store
        .get_session_messages(&session_id, 50)
        .await
        .expect("messages");

    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].content, "hello");
}

#[tokio::test]
async fn default_session_creation_mock_works() {
    let store = InMemoryMessageMock::default();

    let session_id = store
        .get_or_create_default_session("user-3")
        .await
        .expect("session");

    assert_eq!(session_id, "session-user-3");
}
