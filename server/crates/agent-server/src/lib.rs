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

        let database_url = Self::resolve_database_url()?;

        Ok(Self {
            database_url,
            jwt_secret: required_env("JWT_SECRET")?,
            llm_api_key: required_env("LLM_API_KEY")?,
            llm_base_url: required_env("LLM_BASE_URL")?,
            llm_model: required_env("LLM_MODEL")?,
            listen_addr,
        })
    }

    /// Resolve database URL from environment.
    ///
    /// If `DATABASE_URL` is set, use it directly (supports external databases).
    /// Otherwise, build from individual `PG_*` variables with proper percent-encoding.
    fn resolve_database_url() -> Result<String, Box<dyn std::error::Error>> {
        if let Ok(url) = std::env::var("DATABASE_URL") {
            return Ok(url);
        }

        let user = required_env("PG_USER")?;
        let password = required_env("PG_PASSWORD")?;
        let host = std::env::var("PG_HOST").unwrap_or_else(|_| "localhost".to_string());
        let port = std::env::var("PG_PORT").unwrap_or_else(|_| "5432".to_string());
        let database = required_env("PG_DATABASE")?;

        // Percent-encode user and password for URL safety
        let user = percent_encode(&user);
        let password = percent_encode(&password);

        Ok(format!("postgres://{user}:{password}@{host}:{port}/{database}"))
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

/// Percent-encode a string for use in a URI userinfo component (RFC 3986 §3.2.1).
fn percent_encode(input: &str) -> String {
    let mut encoded = String::with_capacity(input.len());
    for byte in input.bytes() {
        match byte {
            // unreserved characters (RFC 3986 §2.3)
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                encoded.push(byte as char);
            }
            _ => {
                encoded.push_str(&format!("%{byte:02X}"));
            }
        }
    }
    encoded
}

fn required_env(key: &str) -> Result<String, std::io::Error> {
    std::env::var(key).map_err(|_| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("{key} environment variable is required"),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percent_encode_preserves_unreserved_chars() {
        assert_eq!(percent_encode("hello"), "hello");
        assert_eq!(percent_encode("user-name_1.0~test"), "user-name_1.0~test");
    }

    #[test]
    fn percent_encode_encodes_special_chars() {
        assert_eq!(percent_encode("p@ss:w/rd#1"), "p%40ss%3Aw%2Frd%231");
        assert_eq!(percent_encode("a b"), "a%20b");
    }

    #[test]
    fn resolve_database_url_prefers_direct_url() {
        // SAFETY: test runs single-threaded (--test-threads=1)
        unsafe {
            std::env::set_var("DATABASE_URL", "postgres://direct:url@host/db");
        }
        let url = ServerConfig::resolve_database_url().unwrap();
        assert_eq!(url, "postgres://direct:url@host/db");
        unsafe {
            std::env::remove_var("DATABASE_URL");
        }
    }

    #[test]
    fn resolve_database_url_builds_from_parts() {
        // SAFETY: test runs single-threaded (--test-threads=1)
        unsafe {
            std::env::remove_var("DATABASE_URL");
            std::env::set_var("PG_USER", "user");
            std::env::set_var("PG_PASSWORD", "p@ss");
            std::env::set_var("PG_HOST", "db.example.com");
            std::env::set_var("PG_PORT", "5433");
            std::env::set_var("PG_DATABASE", "mydb");
        }

        let url = ServerConfig::resolve_database_url().unwrap();
        assert_eq!(url, "postgres://user:p%40ss@db.example.com:5433/mydb");

        unsafe {
            std::env::remove_var("PG_USER");
            std::env::remove_var("PG_PASSWORD");
            std::env::remove_var("PG_HOST");
            std::env::remove_var("PG_PORT");
            std::env::remove_var("PG_DATABASE");
        }
    }
}

/// Test-oriented server builder that accepts injected dependencies (mock LLM, etc.).
pub struct ServerBuilder {
    auth_service: Arc<AuthService>,
    chat_handler: ChatServiceHandler,
    session_handler: SessionServiceHandler,
    addr: SocketAddr,
}

impl ServerBuilder {
    pub fn new(
        user_store: Arc<dyn agent_domain::AuthPort>,
        llm: Arc<dyn LlmProvider>,
        message_store: Arc<dyn agent_domain::MessageStore>,
        jwt_secret: &str,
        addr: SocketAddr,
    ) -> Self {
        let tools = build_tool_runtime();
        let compaction = Arc::new(NoopCompactionService::new());
        let turn_executor = Arc::new(TurnExecutor::new(
            llm,
            tools,
            message_store.clone(),
            compaction,
            TurnExecutorConfig::default(),
        ));
        let auth_service = Arc::new(AuthService::new(user_store, jwt_secret.to_string()));
        let chat_handler = ChatServiceHandler::new(turn_executor);
        let session_handler = SessionServiceHandler::new(message_store);

        Self {
            auth_service,
            chat_handler,
            session_handler,
            addr,
        }
    }

    pub async fn serve_with_shutdown(
        self,
        shutdown: impl std::future::Future<Output = ()>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let auth_handler = AuthServiceHandler::new(self.auth_service.clone());
        let chat_service = ChatServiceServer::with_interceptor(
            self.chat_handler,
            auth_interceptor(self.auth_service.clone()),
        );
        let session_service = SessionServiceServer::with_interceptor(
            self.session_handler,
            auth_interceptor(self.auth_service),
        );
        let auth_service = AuthServiceServer::new(auth_handler);

        Server::builder()
            .add_service(chat_service)
            .add_service(session_service)
            .add_service(auth_service)
            .serve_with_shutdown(self.addr, shutdown)
            .await?;

        Ok(())
    }
}
