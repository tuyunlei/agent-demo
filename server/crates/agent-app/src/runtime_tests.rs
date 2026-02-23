use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use agent_domain::{
    AgentError, ChatMessage, FinishReason, LlmError, LlmProvider, LlmRequest, LlmResponse,
    LlmUsage, MessageStore, StoreError, StoredMessage, ToolCall, ToolResult, ToolRuntime, ToolSpec,
};

use super::*;

struct MockLlmProvider {
    captured: Arc<Mutex<Vec<LlmRequest>>>,
    responses: Arc<Mutex<Vec<Result<LlmResponse, LlmError>>>>,
}

#[async_trait::async_trait]
impl LlmProvider for MockLlmProvider {
    async fn generate(&self, request: LlmRequest) -> Result<LlmResponse, LlmError> {
        self.captured.lock().expect("lock captured").push(request);
        self.responses.lock().expect("responses").remove(0)
    }
}

#[derive(Default)]
struct MockToolRuntime {
    tools: Vec<ToolSpec>,
    results: Arc<Mutex<HashMap<String, String>>>,
}

#[async_trait::async_trait]
impl ToolRuntime for MockToolRuntime {
    fn list_tools(&self) -> Vec<ToolSpec> {
        self.tools.clone()
    }

    async fn execute(&self, name: &str, _arguments: &str) -> Result<ToolResult, AgentError> {
        let content = self
            .results
            .lock()
            .expect("results")
            .get(name)
            .cloned()
            .unwrap_or_else(|| "ok".to_string());
        Ok(ToolResult {
            call_id: String::new(),
            content,
        })
    }
}

#[derive(Default)]
struct MockMessageStore {
    saved: Arc<Mutex<Vec<(String, String, String)>>>,
    history: Arc<Mutex<Vec<StoredMessage>>>,
    default_session: Arc<Mutex<Option<String>>>,
    default_session_result: Arc<Mutex<Option<Result<String, StoreError>>>>,
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

    async fn save_message_ext(
        &self,
        session_id: &str,
        message: &ChatMessage,
    ) -> Result<String, StoreError> {
        self.save_message(session_id, &message.role, &message.content)
            .await
    }

    async fn get_session_messages(
        &self,
        _session_id: &str,
        _limit: i64,
    ) -> Result<Vec<StoredMessage>, StoreError> {
        Ok(self.history.lock().expect("history").clone())
    }

    async fn get_or_create_default_session(&self, _user_id: &str) -> Result<String, StoreError> {
        if let Some(result) = self
            .default_session_result
            .lock()
            .expect("default result")
            .clone()
        {
            return result;
        }

        Ok(self
            .default_session
            .lock()
            .expect("default")
            .clone()
            .unwrap_or_else(|| "default-session".to_string()))
    }
}

fn stop_response(content: &str) -> Result<LlmResponse, LlmError> {
    Ok(LlmResponse {
        content: content.to_string(),
        model: "mock-model".to_string(),
        usage: Some(LlmUsage {
            input_tokens: 1,
            output_tokens: 1,
            total_tokens: 2,
        }),
        tool_calls: vec![],
        finish_reason: FinishReason::Stop,
    })
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
            responses: Arc::new(Mutex::new(vec![stop_response("hello from ai")])),
        }),
        store.clone(),
        Arc::new(MockToolRuntime::default()),
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
async fn handle_message_executes_tools_then_returns_final_text() {
    let captured = Arc::new(Mutex::new(Vec::new()));
    let store = Arc::new(MockMessageStore::default());
    let runtime = AgentRuntime::new(
        Arc::new(MockLlmProvider {
            captured,
            responses: Arc::new(Mutex::new(vec![
                Ok(LlmResponse {
                    content: String::new(),
                    model: "mock".to_string(),
                    usage: None,
                    tool_calls: vec![ToolCall {
                        call_id: "call-1".to_string(),
                        name: "get_current_time".to_string(),
                        arguments: "{}".to_string(),
                    }],
                    finish_reason: FinishReason::ToolCalls,
                }),
                stop_response("final answer"),
            ])),
        }),
        store.clone(),
        Arc::new(MockToolRuntime {
            tools: vec![ToolSpec {
                name: "get_current_time".to_string(),
                description: "time".to_string(),
                parameters: serde_json::json!({"type":"object"}),
            }],
            results: Arc::new(Mutex::new(HashMap::from([(
                "get_current_time".to_string(),
                "2026-01-01T00:00:00+00:00".to_string(),
            )]))),
        }),
    );

    let result = runtime
        .handle_message("u1", Some("s1"), "what time is it")
        .await
        .expect("reply");

    assert_eq!(result.reply, "final answer");
    let saved = store.saved.lock().expect("saved");
    assert_eq!(saved[0].1, "user");
    assert_eq!(saved[1].1, "assistant");
    assert_eq!(saved[2].1, "tool");
    assert_eq!(saved[3].1, "assistant");
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
            responses: Arc::new(Mutex::new(vec![stop_response("ok")])),
        }),
        store,
        Arc::new(MockToolRuntime::default()),
    );

    let result = runtime
        .handle_message("user-1", None, "hello")
        .await
        .expect("ok");

    assert_eq!(result.session_id, "generated-session");
}

#[tokio::test]
async fn handle_message_rejects_empty_content() {
    let runtime = AgentRuntime::new(
        Arc::new(MockLlmProvider {
            captured: Arc::new(Mutex::new(Vec::new())),
            responses: Arc::new(Mutex::new(vec![stop_response("unused")])),
        }),
        Arc::new(MockMessageStore::default()),
        Arc::new(MockToolRuntime::default()),
    );

    let err = runtime
        .handle_message("user-1", Some("s1"), "   \n\t")
        .await
        .unwrap_err();

    assert_eq!(
        err,
        AgentError::InvalidInput("message content cannot be empty".to_string())
    );
}
