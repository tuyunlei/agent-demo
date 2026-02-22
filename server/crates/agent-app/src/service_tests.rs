use super::*;
use agent_domain::{AuthError, AuthPort, AuthResult};
use jsonwebtoken::{EncodingKey, Header, encode};

struct MockAuthPort {
    auth_result: Result<AuthResult, AuthError>,
    create_result: Result<AuthResult, AuthError>,
}

#[async_trait::async_trait]
impl AuthPort for MockAuthPort {
    async fn authenticate(&self, _email: &str, _password: &str) -> Result<AuthResult, AuthError> {
        self.auth_result.clone()
    }

    async fn create_user(
        &self,
        _email: &str,
        _password: &str,
        _display_name: &str,
    ) -> Result<AuthResult, AuthError> {
        self.create_result.clone()
    }
}

fn success_user() -> AuthResult {
    AuthResult {
        user_id: "user-001".to_string(),
        display_name: "Test User".to_string(),
    }
}

fn mock_service() -> AuthService {
    AuthService::new(
        Arc::new(MockAuthPort {
            auth_result: Ok(success_user()),
            create_result: Ok(success_user()),
        }),
        "test-secret".to_string(),
    )
}

#[tokio::test]
async fn login_success_returns_jwt_token_pair() {
    let service = mock_service();
    let result = service
        .login("test@example.com", "password123")
        .await
        .unwrap();

    assert_eq!(result.user_id, "user-001");
    assert!(!result.token_pair.access_token.is_empty());
    assert!(!result.token_pair.refresh_token.is_empty());
}

#[tokio::test]
async fn login_failure_returns_invalid_credentials() {
    let service = AuthService::new(
        Arc::new(MockAuthPort {
            auth_result: Err(AuthError::InvalidCredentials),
            create_result: Ok(success_user()),
        }),
        "test-secret".to_string(),
    );

    let err = service
        .login("wrong@example.com", "wrong-password")
        .await
        .unwrap_err();

    assert_eq!(err, AuthServiceError::InvalidCredentials);
}

#[tokio::test]
async fn register_success_returns_jwt_token_pair() {
    let service = mock_service();

    let result = service
        .register("new@example.com", "password123", "New User")
        .await
        .unwrap();

    assert_eq!(result.user_id, "user-001");
    assert!(!result.token_pair.access_token.is_empty());
    assert!(!result.token_pair.refresh_token.is_empty());
}

#[tokio::test]
async fn register_with_duplicate_email_returns_already_exists() {
    let service = AuthService::new(
        Arc::new(MockAuthPort {
            auth_result: Ok(success_user()),
            create_result: Err(AuthError::AlreadyExists("email already in use".to_string())),
        }),
        "test-secret".to_string(),
    );

    let err = service
        .register("dup@example.com", "password123", "Dup")
        .await
        .unwrap_err();

    assert_eq!(
        err,
        AuthServiceError::AlreadyExists("email already in use".to_string())
    );
}

#[tokio::test]
async fn register_with_empty_email_or_password_returns_invalid_input() {
    let service = mock_service();

    let email_err = service
        .register("", "password123", "User")
        .await
        .unwrap_err();
    assert_eq!(
        email_err,
        AuthServiceError::InvalidInput("email is required".to_string())
    );

    let password_err = service
        .register("u@example.com", "", "User")
        .await
        .unwrap_err();
    assert_eq!(
        password_err,
        AuthServiceError::InvalidInput("password is required".to_string())
    );
}

fn sign_test_token(secret: &str, claims: &Claims) -> String {
    encode(
        &Header::default(),
        claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .expect("sign test token")
}

#[test]
fn validate_token_with_valid_access_token() {
    let service = mock_service();
    let now = current_unix_seconds() as usize;
    let token = sign_test_token(
        "test-secret",
        &Claims {
            sub: "user-001".to_string(),
            exp: now + 3600,
            iat: now,
            token_type: "access".to_string(),
        },
    );

    let claims = service.validate_token(&token).expect("valid token");

    assert_eq!(claims.sub, "user-001");
    assert_eq!(claims.token_type, "access");
}
