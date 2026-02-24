use std::net::SocketAddr;
use std::sync::Arc;

use agent_channel::{
    AuthServiceHandler, ChatServiceHandler, SessionServiceHandler, auth_interceptor,
};
use agent_llm::{LlmProvider, OpenAiProvider};
use agent_memory::NoopCompactionService;
use agent_orchestrator::{AuthService, TurnExecutor, TurnExecutorConfig};
use agent_proto::auth_service_server::AuthServiceServer;
use agent_proto::chat_service_server::ChatServiceServer;
use agent_proto::session_service_server::SessionServiceServer;
use agent_storage::pg::{PostgresEventStore, PostgresMessageStore, PostgresUserStore};
use agent_tools::{DefaultToolRuntime, ToolRuntime};
use sqlx::PgPool;
use tonic::transport::Server;

pub struct ServerConfig {
    pub database_url: String,
    pub jwt_secret: String,
    pub llm_api_key: String,
    pub llm_base_url: String,
    pub llm_model: String,
    pub listen_addr: SocketAddr,
}

impl ServerConfig {
    pub fn from_env() -> Result<Self, Box<dyn std::error::Error>> {
        let listen_addr = std::env::var("LISTEN_ADDR")
            .unwrap_or_else(|_| "127.0.0.1:50051".to_string())
            .parse()?;

        Ok(Self {
            database_url: required_env("DATABASE_URL")?,
            jwt_secret: required_env("JWT_SECRET")?,
            llm_api_key: required_env("LLM_API_KEY")?,
            llm_base_url: required_env("LLM_BASE_URL")?,
            llm_model: required_env("LLM_MODEL")?,
            listen_addr,
        })
    }
}

pub async fn run_server(config: ServerConfig) -> Result<(), Box<dyn std::error::Error>> {
    // Layer 4: Infrastructure
    let pool = PgPool::connect(&config.database_url).await?;
    sqlx::migrate!("../agent-storage/migrations")
        .run(&pool)
        .await?;

    let user_store = Arc::new(PostgresUserStore::new(pool.clone()));
    let message_store = Arc::new(PostgresMessageStore::new(pool.clone()));
    let _event_store = Arc::new(PostgresEventStore::new(pool));

    ensure_admin_user_from_env(&user_store)
        .await
        .map_err(|err| std::io::Error::other(format!("{err:?}")))?;

    // Layer 3: Capabilities
    let llm: Arc<dyn LlmProvider> = Arc::new(OpenAiProvider::with_config(
        config.llm_api_key,
        config.llm_base_url,
        config.llm_model,
    ));
    let tools = build_tool_runtime();
    let compaction = Arc::new(NoopCompactionService::new());

    // Layer 2: Orchestration
    let turn_executor = Arc::new(TurnExecutor::new(
        llm,
        tools,
        message_store.clone(),
        compaction,
        TurnExecutorConfig::default(),
    ));
    let auth_service = Arc::new(AuthService::new(user_store, config.jwt_secret));

    // Layer 1: Channel handlers
    let chat_handler = ChatServiceHandler::new(turn_executor);
    let auth_handler = AuthServiceHandler::new(auth_service.clone());
    let session_handler = SessionServiceHandler::new(message_store);

    // Server assembly
    let chat_service =
        ChatServiceServer::with_interceptor(chat_handler, auth_interceptor(auth_service.clone()));
    let session_service =
        SessionServiceServer::with_interceptor(session_handler, auth_interceptor(auth_service));
    let auth_service = AuthServiceServer::new(auth_handler);

    Server::builder()
        .add_service(chat_service)
        .add_service(session_service)
        .add_service(auth_service)
        .serve(config.listen_addr)
        .await?;

    Ok(())
}

fn build_tool_runtime() -> Arc<dyn ToolRuntime> {
    let mut runtime = DefaultToolRuntime::new();
    runtime.register(Box::new(agent_tools::builtin::GetCurrentTimeTool));
    runtime.register(Box::new(agent_tools::builtin::WebSearchTool::from_env()));
    Arc::new(runtime)
}

async fn ensure_admin_user_from_env(
    user_store: &PostgresUserStore,
) -> Result<(), agent_domain::AuthError> {
    let admin_email =
        std::env::var("ADMIN_EMAIL").unwrap_or_else(|_| "admin@agent-demo.dev".to_string());
    let admin_password = required_env("ADMIN_PASSWORD").map_err(|e| {
        agent_domain::AuthError::Internal(format!(
            "ADMIN_PASSWORD environment variable is required: {e}"
        ))
    })?;

    let existing = user_store.find_by_email(&admin_email).await?;
    if existing.is_none() {
        agent_domain::AuthPort::create_user(
            user_store,
            &admin_email,
            &admin_password,
            "Administrator",
        )
        .await?;
        println!("created admin user: {}", admin_email);
    }

    Ok(())
}

fn required_env(key: &str) -> Result<String, std::io::Error> {
    std::env::var(key).map_err(|_| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("{key} environment variable is required"),
        )
    })
}
