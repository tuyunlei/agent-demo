use std::sync::{Arc, Mutex};

use agent_context::{
    ContextBuilder, ContextError, ContextInput, DefaultContextBuilder, ModelMessage,
    PromptSectionContext,
};
use agent_domain::{ChatMessage, MessageStore, StoreError, StoredMessage};
use agent_llm::mock::MockLlmProvider;
use agent_llm::types::{FinishReason, LlmResponse, LlmUsage, ToolCall};
use agent_memory::NoopCompactionService;
use agent_tools::{
    DefaultToolRuntime, ExecutionClass, Tool, ToolError, ToolInput, ToolOutput, ToolRuntime,
    ToolSpec,
};
use async_trait::async_trait;
use serde_json::json;

use crate::{TurnError, TurnExecutor, TurnExecutorConfig, TurnFinishReason, TurnInput};

#[derive(Default)]
struct MockMessageStore {
    saved: Arc<Mutex<Vec<(String, String, String)>>>,
    saved_ext: Arc<Mutex<Vec<(String, ChatMessage)>>>,
    history: Arc<Mutex<Vec<StoredMessage>>>,
}

#[derive(Default)]
struct FailingMessageStore;

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
        self.saved_ext
            .lock()
            .expect("saved ext")
            .push((session_id.to_string(), message.clone()));
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

    async fn get_or_create_default_session(&self, user_id: &str) -> Result<String, StoreError> {
        Ok(format!("session-{user_id}"))
    }
}

#[async_trait::async_trait]
impl MessageStore for FailingMessageStore {
    async fn create_session(&self, _user_id: &str, _agent_id: &str) -> Result<String, StoreError> {
        Err(StoreError::Internal("boom".to_string()))
    }

    async fn save_message(
        &self,
        _session_id: &str,
        _role: &str,
        _content: &str,
    ) -> Result<String, StoreError> {
        Err(StoreError::Internal("boom".to_string()))
    }

    async fn get_session_messages(
        &self,
        _session_id: &str,
        _limit: i64,
    ) -> Result<Vec<StoredMessage>, StoreError> {
        Ok(vec![])
    }

    async fn get_or_create_default_session(&self, _user_id: &str) -> Result<String, StoreError> {
        Err(StoreError::Internal("boom".to_string()))
    }
}

struct EchoTool;

#[async_trait]
impl Tool for EchoTool {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "echo".to_string(),
            description: "echo".to_string(),
            parameters_schema: json!({"type": "object"}),
            strict: false,
            execution_class: ExecutionClass::Local,
        }
    }

    async fn execute(&self, input: ToolInput) -> Result<ToolOutput, ToolError> {
        Ok(ToolOutput {
            content_json: input.arguments_json,
        })
    }
}

struct StaticContextBuilder {
    system_prompt: String,
}

impl ContextBuilder for StaticContextBuilder {
    fn build_system_prompt(&self, _ctx: &PromptSectionContext) -> Result<String, ContextError> {
        Ok(self.system_prompt.clone())
    }

    fn build_messages(&self, _input: &ContextInput) -> Result<Vec<ModelMessage>, ContextError> {
        Ok(Vec::new())
    }
}

fn response_with(
    reason: FinishReason,
    content: &str,
    tool_calls: Vec<ToolCall>,
) -> Result<LlmResponse, agent_llm::LlmError> {
    Ok(LlmResponse {
        content: content.to_string(),
        tool_calls,
        finish_reason: reason,
        usage: Some(LlmUsage {
            prompt_tokens: 1,
            completion_tokens: 1,
            total_tokens: 2,
            cache_read_tokens: None,
            cache_write_tokens: None,
        }),
        model: "mock".to_string(),
        provider: "mock".to_string(),
        response_id: None,
    })
}

#[tokio::test]
async fn simple_text_reply() {
    let llm = Arc::new(MockLlmProvider::new(response_with(
        FinishReason::Stop,
        "hello",
        vec![],
    )));
    let llm_for_assert = llm.clone();
    let store = Arc::new(MockMessageStore::default());
    let runtime = TurnExecutor::new(
        llm,
        Arc::new(DefaultToolRuntime::new()),
        store,
        Arc::new(NoopCompactionService::new()),
        Arc::new(StaticContextBuilder {
            system_prompt: "system-from-context-builder".to_string(),
        }),
        TurnExecutorConfig::default(),
    );

    let out = runtime
        .run_turn(TurnInput {
            user_id: "u1".to_string(),
            session_id: Some("s1".to_string()),
            user_message: "hi".to_string(),
        })
        .await
        .expect("ok");

    assert_eq!(out.session_id, "s1");
    assert_eq!(out.assistant_text, "hello");
    assert_eq!(out.finish_reason, TurnFinishReason::Stop);
    assert_eq!(
        llm_for_assert.calls()[0].messages[0].content,
        "system-from-context-builder"
    );
}

#[tokio::test]
async fn empty_message_rejected() {
    let runtime = TurnExecutor::new(
        Arc::new(MockLlmProvider::with_text("unused")),
        Arc::new(DefaultToolRuntime::new()),
        Arc::new(MockMessageStore::default()),
        Arc::new(NoopCompactionService::new()),
        Arc::new(DefaultContextBuilder::with_default_sections()),
        TurnExecutorConfig::default(),
    );

    let err = runtime
        .run_turn(TurnInput {
            user_id: "u1".to_string(),
            session_id: None,
            user_message: "  \n".to_string(),
        })
        .await
        .unwrap_err();

    assert!(matches!(err, TurnError::InvalidInput(_)));
}

#[tokio::test]
async fn tool_call_round_trip() {
    let llm = Arc::new(MockLlmProvider::with_tool_call_sequence(vec![
        response_with(
            FinishReason::ToolCalls,
            "",
            vec![ToolCall {
                call_id: "c1".to_string(),
                name: "echo".to_string(),
                arguments: "{\"x\":1}".to_string(),
            }],
        ),
        response_with(FinishReason::Stop, "done", vec![]),
    ]));

    let mut tools = DefaultToolRuntime::new();
    tools.register(Box::new(EchoTool));

    let store = Arc::new(MockMessageStore::default());
    let runtime = TurnExecutor::new(
        llm,
        Arc::new(tools),
        store.clone(),
        Arc::new(NoopCompactionService::new()),
        Arc::new(DefaultContextBuilder::with_default_sections()),
        TurnExecutorConfig::default(),
    );

    let out = runtime
        .run_turn(TurnInput {
            user_id: "u1".to_string(),
            session_id: Some("s1".to_string()),
            user_message: "do it".to_string(),
        })
        .await
        .expect("ok");

    assert_eq!(out.assistant_text, "done");
    assert_eq!(out.tool_iterations, 1);
    assert_eq!(store.saved_ext.lock().expect("saved ext").len(), 2);
}

#[tokio::test]
async fn tool_loop_exceeded() {
    let llm = Arc::new(MockLlmProvider::with_tool_call_sequence(vec![
        response_with(
            FinishReason::ToolCalls,
            "",
            vec![ToolCall {
                call_id: "c1".to_string(),
                name: "echo".to_string(),
                arguments: "{}".to_string(),
            }],
        ),
        response_with(
            FinishReason::ToolCalls,
            "",
            vec![ToolCall {
                call_id: "c2".to_string(),
                name: "echo".to_string(),
                arguments: "{}".to_string(),
            }],
        ),
    ]));

    let mut tools = DefaultToolRuntime::new();
    tools.register(Box::new(EchoTool));

    let runtime = TurnExecutor::new(
        llm,
        Arc::new(tools),
        Arc::new(MockMessageStore::default()),
        Arc::new(NoopCompactionService::new()),
        Arc::new(DefaultContextBuilder::with_default_sections()),
        TurnExecutorConfig {
            max_tool_iterations: 1,
            timezone: "Asia/Shanghai".to_string(),
        },
    );

    let err = runtime
        .run_turn(TurnInput {
            user_id: "u1".to_string(),
            session_id: Some("s1".to_string()),
            user_message: "loop".to_string(),
        })
        .await
        .unwrap_err();

    assert_eq!(err, TurnError::ToolLoopExceeded { max: 1 });
}

#[tokio::test]
async fn length_truncated() {
    let llm = Arc::new(MockLlmProvider::new(response_with(
        FinishReason::Length,
        "partial",
        vec![],
    )));
    let runtime = TurnExecutor::new(
        llm,
        Arc::new(DefaultToolRuntime::new()),
        Arc::new(MockMessageStore::default()),
        Arc::new(NoopCompactionService::new()),
        Arc::new(DefaultContextBuilder::with_default_sections()),
        TurnExecutorConfig::default(),
    );

    let out = runtime
        .run_turn(TurnInput {
            user_id: "u1".to_string(),
            session_id: Some("s1".to_string()),
            user_message: "long".to_string(),
        })
        .await
        .expect("ok");

    assert_eq!(out.assistant_text, "partial");
    assert_eq!(out.finish_reason, TurnFinishReason::LengthTruncated);
}

#[tokio::test]
async fn finish_reason_error_and_content_filter_fallback_to_stop() {
    for reason in [FinishReason::Error, FinishReason::ContentFilter] {
        let runtime = TurnExecutor::new(
            Arc::new(MockLlmProvider::new(response_with(
                reason,
                "fallback",
                vec![],
            ))),
            Arc::new(DefaultToolRuntime::new()),
            Arc::new(MockMessageStore::default()),
            Arc::new(NoopCompactionService::new()),
            Arc::new(DefaultContextBuilder::with_default_sections()),
            TurnExecutorConfig::default(),
        );

        let out = runtime
            .run_turn(TurnInput {
                user_id: "u1".to_string(),
                session_id: Some("s1".to_string()),
                user_message: "hi".to_string(),
            })
            .await
            .expect("ok");

        assert_eq!(out.finish_reason, TurnFinishReason::Stop);
        assert_eq!(out.assistant_text, "fallback");
    }
}

#[tokio::test]
async fn resolves_default_session_when_input_session_id_missing_or_blank() {
    let llm = Arc::new(MockLlmProvider::with_text("ok"));
    let runtime = TurnExecutor::new(
        llm,
        Arc::new(DefaultToolRuntime::new()),
        Arc::new(MockMessageStore::default()),
        Arc::new(NoopCompactionService::new()),
        Arc::new(DefaultContextBuilder::with_default_sections()),
        TurnExecutorConfig::default(),
    );

    let out_missing = runtime
        .run_turn(TurnInput {
            user_id: "u1".to_string(),
            session_id: None,
            user_message: "hi".to_string(),
        })
        .await
        .expect("ok");
    assert_eq!(out_missing.session_id, "session-u1");

    let out_blank = runtime
        .run_turn(TurnInput {
            user_id: "u1".to_string(),
            session_id: Some("   ".to_string()),
            user_message: "hi again".to_string(),
        })
        .await
        .expect("ok");
    assert_eq!(out_blank.session_id, "session-u1");
}

#[tokio::test]
async fn store_errors_are_mapped_to_session_error() {
    let runtime = TurnExecutor::new(
        Arc::new(MockLlmProvider::with_text("unused")),
        Arc::new(DefaultToolRuntime::new()),
        Arc::new(FailingMessageStore),
        Arc::new(NoopCompactionService::new()),
        Arc::new(DefaultContextBuilder::with_default_sections()),
        TurnExecutorConfig::default(),
    );

    let err = runtime
        .run_turn(TurnInput {
            user_id: "u1".to_string(),
            session_id: None,
            user_message: "hello".to_string(),
        })
        .await
        .expect_err("store should fail");

    assert!(matches!(err, TurnError::SessionError(_)));
}
