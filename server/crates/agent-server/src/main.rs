use std::sync::Arc;

use agent_app::AuthService;
use agent_channel::{AuthServiceHandler, ChatServiceHandler, auth_interceptor};
use agent_domain::{AuthError, AuthPort, AuthResult};
use agent_proto::auth_service_server::AuthServiceServer;
use agent_proto::chat_service_server::ChatServiceServer;
use tonic::transport::Server;

struct EnvAuthProvider {
    email: String,
    password: String,
}

#[async_trait::async_trait]
impl AuthPort for EnvAuthProvider {
    async fn authenticate(&self, email: &str, password: &str) -> Result<AuthResult, AuthError> {
        if email == self.email && password == self.password {
            Ok(AuthResult {
                user_id: "user-001".to_string(),
                display_name: "Test User".to_string(),
            })
        } else {
            Err(AuthError::InvalidCredentials)
        }
    }
}

fn required_env(key: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| panic!("{} environment variable is required", key))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "127.0.0.1:50051".parse()?;
    println!("agent-server listening on {}", addr);

    let jwt_secret = required_env("JWT_SECRET");
    let admin_email = std::env::var("ADMIN_EMAIL").unwrap_or_else(|_| "admin@agent-demo.dev".to_string());
    let admin_password = required_env("ADMIN_PASSWORD");

    let auth_provider = Arc::new(EnvAuthProvider {
        email: admin_email,
        password: admin_password,
    });
    let auth_service = Arc::new(AuthService::new(
        auth_provider,
        jwt_secret,
    ));

    let chat_service = ChatServiceServer::with_interceptor(
        ChatServiceHandler,
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
