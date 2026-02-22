use std::sync::Arc;

use agent_app::{AgentRuntime, AuthService};
use agent_channel::{AuthServiceHandler, ChatServiceHandler, auth_interceptor};
use agent_llm::OpenAiProvider;
use agent_proto::auth_service_server::AuthServiceServer;
use agent_proto::chat_service_server::ChatServiceServer;
use agent_storage::pg::PostgresUserStore;
use sqlx::postgres::PgPoolOptions;
use tonic::transport::Server;

fn required_env(key: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| panic!("{} environment variable is required", key))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "127.0.0.1:50051".parse()?;
    println!("agent-server listening on {}", addr);

    let jwt_secret = required_env("JWT_SECRET");
    let admin_email =
        std::env::var("ADMIN_EMAIL").unwrap_or_else(|_| "admin@agent-demo.dev".to_string());
    let admin_password = required_env("ADMIN_PASSWORD");
    let database_url = required_env("DATABASE_URL");

    let llm_base_url = required_env("LLM_BASE_URL");
    let llm_api_key = required_env("LLM_API_KEY");
    let llm_model = required_env("LLM_MODEL");

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    sqlx::migrate!("../agent-storage/migrations")
        .run(&pool)
        .await?;

    let user_store = Arc::new(PostgresUserStore::new(pool));
    ensure_admin_user(&user_store, &admin_email, &admin_password)
        .await
        .map_err(|err| std::io::Error::other(format!("{err:?}")))?;

    let auth_service = Arc::new(AuthService::new(user_store, jwt_secret));

    let llm_provider = Arc::new(OpenAiProvider::with_config(
        llm_api_key,
        llm_base_url,
        llm_model,
    ));
    let runtime = Arc::new(AgentRuntime::new(llm_provider));

    let chat_service = ChatServiceServer::with_interceptor(
        ChatServiceHandler::new(runtime),
        auth_interceptor(auth_service.clone()),
    );
    let auth_service = AuthServiceServer::new(AuthServiceHandler::new(auth_service));

    Server::builder()
        .add_service(chat_service)
        .add_service(auth_service)
        .serve(addr)
        .await?;

    Ok(())
}

async fn ensure_admin_user(
    user_store: &PostgresUserStore,
    admin_email: &str,
    admin_password: &str,
) -> Result<(), agent_domain::AuthError> {
    let existing = user_store.find_by_email(admin_email).await?;
    if existing.is_none() {
        user_store
            .create_user(admin_email, admin_password, "Administrator")
            .await?;
        println!("created admin user: {}", admin_email);
    }

    Ok(())
}
