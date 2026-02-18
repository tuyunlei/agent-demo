use async_trait::async_trait;
use std::sync::Arc;

pub type Token = String;
pub type AppResult<T> = Result<T, String>;

// ---- Port traits (hexagonal architecture) ----
#[async_trait]
pub trait LlmProvider: Send + Sync {
    async fn stream_tokens(&self, prompt: &str) -> AppResult<Vec<Token>>;
}

#[async_trait]
pub trait MessageStore: Send + Sync {
    async fn append_message(&self, session_id: &str, message: &str) -> AppResult<()>;
}

#[async_trait]
pub trait SessionStore: Send + Sync {
    async fn load_session(&self, session_id: &str) -> AppResult<Option<String>>;
}

// ---- Option A: dyn Trait (object-safe via async_trait boxed future) ----
pub struct SendMessageServiceDyn {
    llm: Arc<dyn LlmProvider>,
    messages: Arc<dyn MessageStore>,
    sessions: Arc<dyn SessionStore>,
}

impl SendMessageServiceDyn {
    pub fn new(
        llm: Arc<dyn LlmProvider>,
        messages: Arc<dyn MessageStore>,
        sessions: Arc<dyn SessionStore>,
    ) -> Self {
        Self {
            llm,
            messages,
            sessions,
        }
    }

    pub async fn send(&self, session_id: &str, prompt: &str) -> AppResult<Vec<Token>> {
        let _existing = self.sessions.load_session(session_id).await?;
        self.messages
            .append_message(session_id, prompt)
            .await?;
        self.llm.stream_tokens(prompt).await
    }
}

// ---- Option B: generic parameters (static dispatch) ----
pub struct SendMessageServiceGeneric<L, M, S>
where
    L: LlmProvider,
    M: MessageStore,
    S: SessionStore,
{
    llm: L,
    messages: M,
    sessions: S,
}

impl<L, M, S> SendMessageServiceGeneric<L, M, S>
where
    L: LlmProvider,
    M: MessageStore,
    S: SessionStore,
{
    pub fn new(llm: L, messages: M, sessions: S) -> Self {
        Self {
            llm,
            messages,
            sessions,
        }
    }

    pub async fn send(&self, session_id: &str, prompt: &str) -> AppResult<Vec<Token>> {
        let _existing = self.sessions.load_session(session_id).await?;
        self.messages
            .append_message(session_id, prompt)
            .await?;
        self.llm.stream_tokens(prompt).await
    }
}

// ---- Fake adapters for compile verification ----
#[derive(Default)]
pub struct FakeLlm;

#[async_trait]
impl LlmProvider for FakeLlm {
    async fn stream_tokens(&self, prompt: &str) -> AppResult<Vec<Token>> {
        Ok(vec![format!("echo:{prompt}")])
    }
}

#[derive(Default)]
pub struct FakeMessageStore;

#[async_trait]
impl MessageStore for FakeMessageStore {
    async fn append_message(&self, _session_id: &str, _message: &str) -> AppResult<()> {
        Ok(())
    }
}

#[derive(Default)]
pub struct FakeSessionStore;

#[async_trait]
impl SessionStore for FakeSessionStore {
    async fn load_session(&self, _session_id: &str) -> AppResult<Option<String>> {
        Ok(Some("mock-session".to_string()))
    }
}

pub fn build_dyn_service() -> SendMessageServiceDyn {
    SendMessageServiceDyn::new(
        Arc::new(FakeLlm),
        Arc::new(FakeMessageStore),
        Arc::new(FakeSessionStore),
    )
}

pub fn build_generic_service() -> SendMessageServiceGeneric<FakeLlm, FakeMessageStore, FakeSessionStore> {
    SendMessageServiceGeneric::new(FakeLlm, FakeMessageStore, FakeSessionStore)
}
