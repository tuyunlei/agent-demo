use std::sync::Arc;

use agent_app::AuthService;
use agent_channel::{AuthServiceHandler, ChatServiceHandler};
use agent_domain::{AuthError, AuthPort, AuthResult};
use agent_proto::auth_service_server::AuthServiceServer;
use agent_proto::chat_service_server::ChatServiceServer;
use tonic::transport::Server;

struct HardcodedAuthProvider;

#[async_trait::async_trait]
impl AuthPort for HardcodedAuthProvider {
    async fn authenticate(&self, email: &str, password: &str) -> Result<AuthResult, AuthError> {
        if email == "test@example.com" && password == "password123" {
            Ok(AuthResult {
                user_id: "user-001".to_string(),
                display_name: "Test User".to_string(),
            })
        } else {
            Err(AuthError::InvalidCredentials)
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "[::1]:50051".parse()?;
    println!("agent-server listening on {}", addr);

    let auth_provider = Arc::new(HardcodedAuthProvider);
    let auth_service = Arc::new(AuthService::new(
        auth_provider,
        "dev-secret-do-not-use-in-prod".to_string(),
    ));

    Server::builder()
        .add_service(ChatServiceServer::new(ChatServiceHandler))
        .add_service(AuthServiceServer::new(AuthServiceHandler::new(
            auth_service,
        )))
        .serve(addr)
        .await?;

    Ok(())
}
