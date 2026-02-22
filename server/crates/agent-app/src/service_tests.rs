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
async fn login_access_token_expires_in_one_hour() {
    let service = mock_service();
    let result = service
        .login("test@example.com", "password123")
        .await
        .expect("login success");

    let now = current_unix_seconds();
    let access_exp = result.token_pair.access_token_expires_at;

    assert!(
        access_exp > now + 3500,
        "access token should expire in ~1 hour, got {}",
        access_exp - now
    );
    assert!(
        access_exp < now + 3700,
        "access token should expire in ~1 hour, got {}",
        access_exp - now
    );
}

#[tokio::test]
async fn login_refresh_token_expires_in_seven_days() {
    let service = mock_service();
    let result = service
        .login("test@example.com", "password123")
        .await
        .expect("login success");

    let now = current_unix_seconds();
    let refresh_exp = result.token_pair.refresh_token_expires_at;
    let seven_days = 7 * 24 * 60 * 60;

    assert!(
        refresh_exp > now + seven_days - 100,
        "refresh token should expire in ~7 days, got {}",
        refresh_exp - now
    );
    assert!(
        refresh_exp < now + seven_days + 100,
        "refresh token should expire in ~7 days, got {}",
        refresh_exp - now
    );
}

#[tokio::test]
async fn login_tokens_are_valid_jwt() {
    let service = mock_service();
    let result = service
        .login("test@example.com", "password123")
        .await
        .expect("login success");

    let access_claims = service
        .validate_token(&result.token_pair.access_token)
        .expect("access token is valid jwt");
    assert_eq!(access_claims.sub, result.user_id);
    assert_eq!(access_claims.token_type, "access");

    let refresh_claims = service
        .validate_token(&result.token_pair.refresh_token)
        .expect("refresh token is valid jwt");
    assert_eq!(refresh_claims.sub, result.user_id);
    assert_eq!(refresh_claims.token_type, "refresh");
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

#[tokio::test]
async fn register_whitespace_email_returns_invalid_input() {
    let service = mock_service();

    let err = service
        .register("   \n\t", "password123", "User")
        .await
        .unwrap_err();

    assert_eq!(
        err,
        AuthServiceError::InvalidInput("email is required".to_string())
    );
}

#[tokio::test]
async fn login_auth_port_internal_error() {
    let service = AuthService::new(
        Arc::new(MockAuthPort {
            auth_result: Err(AuthError::Internal("db unavailable".to_string())),
            create_result: Ok(success_user()),
        }),
        "test-secret".to_string(),
    );

    let err = service
        .login("test@example.com", "password123")
        .await
        .unwrap_err();

    assert_eq!(
        err,
        AuthServiceError::Internal("db unavailable".to_string())
    );
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
