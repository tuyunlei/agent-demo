use super::*;
use agent_domain::{AuthError, AuthPort, AuthResult};
use jsonwebtoken::{EncodingKey, Header, encode};
use std::sync::Arc;

struct MockAuthPort;

#[async_trait::async_trait]
impl AuthPort for MockAuthPort {
    async fn authenticate(&self, _email: &str, _password: &str) -> Result<AuthResult, AuthError> {
        Ok(success_user())
    }

    async fn create_user(
        &self,
        _email: &str,
        _password: &str,
        _display_name: &str,
    ) -> Result<AuthResult, AuthError> {
        Ok(success_user())
    }
}

fn success_user() -> AuthResult {
    AuthResult {
        user_id: "user-001".to_string(),
        display_name: "Test User".to_string(),
    }
}

fn mock_service() -> AuthService {
    AuthService::new(Arc::new(MockAuthPort), "test-secret".to_string())
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
fn validate_token_expired_returns_error() {
    let service = mock_service();
    let now = current_unix_seconds() as usize;
    let token = sign_test_token(
        "test-secret",
        &Claims {
            sub: "user-001".to_string(),
            exp: now.saturating_sub(120),
            iat: now.saturating_sub(7200),
            token_type: "access".to_string(),
        },
    );

    let err = service.validate_token(&token).unwrap_err();
    assert_eq!(err, AuthServiceError::TokenValidation);
}

#[test]
fn validate_token_invalid_returns_error() {
    let service = mock_service();

    let err = service.validate_token("this-is-not-a-jwt").unwrap_err();
    assert_eq!(err, AuthServiceError::TokenValidation);
}

#[test]
fn validate_token_wrong_secret_returns_error() {
    let service = mock_service();
    let now = current_unix_seconds() as usize;
    let token = sign_test_token(
        "another-secret",
        &Claims {
            sub: "user-001".to_string(),
            exp: now + 3600,
            iat: now,
            token_type: "access".to_string(),
        },
    );

    let err = service.validate_token(&token).unwrap_err();
    assert_eq!(err, AuthServiceError::TokenValidation);
}

#[tokio::test]
async fn refresh_with_valid_refresh_token_returns_new_token_pair() {
    let service = mock_service();
    let login_result = service
        .login("test@example.com", "password123")
        .await
        .expect("login success");

    let refreshed = service
        .refresh_token(&login_result.token_pair.refresh_token)
        .await
        .expect("refresh success");

    assert_eq!(refreshed.user_id, login_result.user_id);
    assert!(!refreshed.token_pair.access_token.is_empty());
    assert!(!refreshed.token_pair.refresh_token.is_empty());
}

#[tokio::test]
async fn refresh_with_access_token_returns_invalid_input() {
    let service = mock_service();
    let login_result = service
        .login("test@example.com", "password123")
        .await
        .expect("login success");

    let err = service
        .refresh_token(&login_result.token_pair.access_token)
        .await
        .unwrap_err();

    assert_eq!(
        err,
        AuthServiceError::InvalidInput("not a refresh token".to_string())
    );
}

#[tokio::test]
async fn refresh_with_expired_token_returns_error() {
    let service = mock_service();
    let now = current_unix_seconds() as usize;
    let expired_refresh = sign_test_token(
        "test-secret",
        &Claims {
            sub: "user-001".to_string(),
            exp: now.saturating_sub(120),
            iat: now.saturating_sub(7200),
            token_type: "refresh".to_string(),
        },
    );

    let err = service.refresh_token(&expired_refresh).await.unwrap_err();

    assert_eq!(err, AuthServiceError::TokenValidation);
}

#[tokio::test]
async fn refresh_with_invalid_token_returns_error() {
    let service = mock_service();

    let err = service
        .refresh_token("not-a-jwt")
        .await
        .expect_err("refresh should fail");

    assert_eq!(err, AuthServiceError::TokenValidation);
}
