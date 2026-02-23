use super::*;
use agent_domain::{AuthError, AuthPort, AuthResult};

struct MockAuthPort;

#[async_trait::async_trait]
impl AuthPort for MockAuthPort {
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

    async fn create_user(
        &self,
        email: &str,
        _password: &str,
        display_name: &str,
    ) -> Result<AuthResult, AuthError> {
        if email == "taken@example.com" {
            return Err(AuthError::AlreadyExists("email already in use".to_string()));
        }
        Ok(AuthResult {
            user_id: "user-001".to_string(),
            display_name: display_name.to_string(),
        })
    }
}

#[tokio::test]
async fn login_success_returns_token_pair() {
    let auth_service = Arc::new(AuthService::new(
        Arc::new(MockAuthPort),
        "test-secret".to_string(),
    ));
    let handler = AuthServiceHandler::new(auth_service);

    let request = Request::new(LoginRequest {
        email: "test@example.com".to_string(),
        password: "password123".to_string(),
        device_name: "dev".to_string(),
        platform: "linux".to_string(),
    });

    let response = handler.login(request).await.unwrap().into_inner();

    assert_eq!(response.user_id, "user-001");
    assert!(response.token_pair.is_some());
}

#[tokio::test]
async fn login_failure_returns_unauthenticated() {
    let auth_service = Arc::new(AuthService::new(
        Arc::new(MockAuthPort),
        "test-secret".to_string(),
    ));
    let handler = AuthServiceHandler::new(auth_service);

    let request = Request::new(LoginRequest {
        email: "wrong@example.com".to_string(),
        password: "wrong-password".to_string(),
        device_name: "dev".to_string(),
        platform: "linux".to_string(),
    });

    let err = handler.login(request).await.unwrap_err();
    assert_eq!(err.code(), tonic::Code::Unauthenticated);
}

#[tokio::test]
async fn register_success_returns_token_pair() {
    let auth_service = Arc::new(AuthService::new(
        Arc::new(MockAuthPort),
        "test-secret".to_string(),
    ));
    let handler = AuthServiceHandler::new(auth_service);

    let request = Request::new(RegisterRequest {
        email: "new@example.com".to_string(),
        password: "password123".to_string(),
        display_name: "New User".to_string(),
        invite_code: "ignored".to_string(),
    });

    let response = handler.register(request).await.unwrap().into_inner();

    assert_eq!(response.user_id, "user-001");
    assert!(response.token_pair.is_some());
}

#[tokio::test]
async fn register_duplicate_email_returns_already_exists() {
    let auth_service = Arc::new(AuthService::new(
        Arc::new(MockAuthPort),
        "test-secret".to_string(),
    ));
    let handler = AuthServiceHandler::new(auth_service);

    let request = Request::new(RegisterRequest {
        email: "taken@example.com".to_string(),
        password: "password123".to_string(),
        display_name: "Dup User".to_string(),
        invite_code: "ignored".to_string(),
    });

    let err = handler.register(request).await.unwrap_err();
    assert_eq!(err.code(), tonic::Code::AlreadyExists);
}

#[tokio::test]
async fn refresh_token_success_returns_new_token_pair() {
    let auth_service = Arc::new(AuthService::new(
        Arc::new(MockAuthPort),
        "test-secret".to_string(),
    ));
    let handler = AuthServiceHandler::new(Arc::clone(&auth_service));

    let login = auth_service
        .login("test@example.com", "password123")
        .await
        .expect("login should succeed");

    let request = Request::new(RefreshTokenRequest {
        refresh_token: login.token_pair.refresh_token,
    });

    let response = handler.refresh_token(request).await.unwrap().into_inner();

    let token_pair = response.token_pair.expect("token pair should exist");
    assert!(!token_pair.access_token.is_empty());
    assert!(!token_pair.refresh_token.is_empty());
    assert!(token_pair.access_token_expires_at.is_some());
    assert!(token_pair.refresh_token_expires_at.is_some());
}
