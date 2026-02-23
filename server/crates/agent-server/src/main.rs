use std::sync::Arc;

use agent_llm::OpenAiProvider;
use agent_server::ServerBuilder;
use agent_storage::pg::{PostgresMessageStore, PostgresUserStore};
use sqlx::postgres::PgPoolOptions;

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

    let user_store = Arc::new(PostgresUserStore::new(pool.clone()));
    let message_store = Arc::new(PostgresMessageStore::new(pool));
    ensure_admin_user(&user_store, &admin_email, &admin_password)
        .await
        .map_err(|err| std::io::Error::other(format!("{err:?}")))?;

    let llm_provider = Arc::new(OpenAiProvider::with_config(
        llm_api_key,
        llm_base_url,
        llm_model,
    ));

    ServerBuilder::new(user_store, llm_provider, message_store, jwt_secret, addr)
        .serve()
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
        agent_domain::AuthPort::create_user(
            user_store,
            admin_email,
            admin_password,
            "Administrator",
        )
        .await?;
        println!("created admin user: {}", admin_email);
    }

    Ok(())
}
