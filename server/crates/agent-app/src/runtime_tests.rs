use std::sync::{Arc, Mutex};

use agent_domain::{LlmError, LlmResponse, LlmUsage, StoreError, StoredMessage};

use super::*;

struct MockLlmProvider {
    captured: Arc<Mutex<Vec<LlmRequest>>>,
    response: Result<LlmResponse, LlmError>,
}

#[async_trait::async_trait]
impl LlmProvider for MockLlmProvider {
    async fn generate(&self, request: LlmRequest) -> Result<LlmResponse, LlmError> {
        self.captured.lock().expect("lock captured").push(request);
        self.response.clone()
    }
}

#[derive(Default)]
struct MockMessageStore {
    saved: Arc<Mutex<Vec<(String, String, String)>>>,
    history: Arc<Mutex<Vec<StoredMessage>>>,
    default_session: Arc<Mutex<Option<String>>>,
}

#[async_trait::async_trait]
impl MessageStore for MockMessageStore {
    async fn create_session(&self, user_id: &str, _agent_id: &str) -> Result<String, StoreError> {
        Ok(format!("session-{user_id}"))
    }

    async fn save_message(
        &self,
        session_id: &str,
        role: &str,
        content: &str,
    ) -> Result<String, StoreError> {
        self.saved.lock().expect("saved").push((
            session_id.to_string(),
            role.to_string(),
            content.to_string(),
        ));
        Ok("msg-id".to_string())
    }

    async fn get_session_messages(
        &self,
        _session_id: &str,
        _limit: i64,
    ) -> Result<Vec<StoredMessage>, StoreError> {
        Ok(self.history.lock().expect("history").clone())
    }

    async fn get_or_create_default_session(&self, _user_id: &str) -> Result<String, StoreError> {
        Ok(self
            .default_session
            .lock()
            .expect("default")
            .clone()
            .unwrap_or_else(|| "default-session".to_string()))
    }
}

#[tokio::test]
async fn handle_message_with_store_persists_and_uses_history() {
    let captured = Arc::new(Mutex::new(Vec::new()));
    let store = Arc::new(MockMessageStore {
        history: Arc::new(Mutex::new(vec![StoredMessage {
            id: "m1".to_string(),
            session_id: "s1".to_string(),
            role: "user".to_string(),
            content: "old message".to_string(),
            created_at: 1,
        }])),
        ..Default::default()
    });

    let runtime = AgentRuntime::new(
        Arc::new(MockLlmProvider {
            captured: captured.clone(),
            response: Ok(LlmResponse {
                content: "hello from ai".to_string(),
                model: "mock-model".to_string(),
                usage: Some(LlmUsage {
                    input_tokens: 1,
                    output_tokens: 1,
                    total_tokens: 2,
                }),
            }),
        }),
        store.clone(),
    );

    let result = runtime
        .handle_message("u1", Some("s1"), "new message")
        .await
        .expect("reply");

    assert_eq!(result.session_id, "s1");
    assert_eq!(result.reply, "hello from ai");

    let requests = captured.lock().expect("lock captured");
    assert_eq!(requests[0].messages[0].role, "system");
    assert_eq!(requests[0].messages[1].content, "old message");

    let saved = store.saved.lock().expect("saved");
    assert_eq!(saved.len(), 2);
    assert_eq!(saved[0].1, "user");
    assert_eq!(saved[1].1, "assistant");
}

#[tokio::test]
async fn handle_message_creates_default_session_when_missing() {
    let store = Arc::new(MockMessageStore {
        default_session: Arc::new(Mutex::new(Some("generated-session".to_string()))),
        ..Default::default()
    });

    let runtime = AgentRuntime::new(
        Arc::new(MockLlmProvider {
            captured: Arc::new(Mutex::new(Vec::new())),
            response: Ok(LlmResponse {
                content: "ok".to_string(),
                model: "mock-model".to_string(),
                usage: None,
            }),
        }),
        store,
    );

    let result = runtime
        .handle_message("user-1", None, "hello")
        .await
        .expect("ok");

    assert_eq!(result.session_id, "generated-session");
}
