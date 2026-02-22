use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use agent_domain::{AuthError, AuthPort};
use agent_types::types::TokenPair;
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};

const ACCESS_TOKEN_TTL_SECONDS: i64 = 60 * 60;
const REFRESH_TOKEN_TTL_SECONDS: i64 = 7 * 24 * 60 * 60;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub exp: usize,
    pub iat: usize,
    pub token_type: String,
}

pub struct AuthService {
    auth_port: Arc<dyn AuthPort>,
    jwt_secret: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthServiceError {
    InvalidCredentials,
    AlreadyExists(String),
    InvalidInput(String),
    TokenCreation,
    TokenValidation,
    Internal(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoginResult {
    pub user_id: String,
    pub token_pair: TokenPair,
}

impl AuthService {
    pub fn new(auth_port: Arc<dyn AuthPort>, jwt_secret: String) -> Self {
        Self {
            auth_port,
            jwt_secret,
        }
    }

    pub async fn login(
        &self,
        email: &str,
        password: &str,
    ) -> Result<LoginResult, AuthServiceError> {
        let user = self
            .auth_port
            .authenticate(email, password)
            .await
            .map_err(map_auth_error)?;

        let token_pair = self.create_token_pair(&user.user_id)?;

        Ok(LoginResult {
            user_id: user.user_id,
            token_pair,
        })
    }

    pub async fn register(
        &self,
        email: &str,
        password: &str,
        display_name: &str,
    ) -> Result<LoginResult, AuthServiceError> {
        if email.trim().is_empty() {
            return Err(AuthServiceError::InvalidInput(
                "email is required".to_string(),
            ));
        }
        if password.trim().is_empty() {
            return Err(AuthServiceError::InvalidInput(
                "password is required".to_string(),
            ));
        }

        let user = self
            .auth_port
            .create_user(email, password, display_name)
            .await
            .map_err(map_auth_error)?;

        let token_pair = self.create_token_pair(&user.user_id)?;

        Ok(LoginResult {
            user_id: user.user_id,
            token_pair,
        })
    }

    pub fn validate_token(&self, token: &str) -> Result<Claims, AuthServiceError> {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.validate_exp = true;

        let token_data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(self.jwt_secret.as_bytes()),
            &validation,
        )
        .map_err(|_| AuthServiceError::TokenValidation)?;

        Ok(token_data.claims)
    }

    fn create_token_pair(&self, user_id: &str) -> Result<TokenPair, AuthServiceError> {
        let now = current_unix_seconds();
        let access_exp = now + ACCESS_TOKEN_TTL_SECONDS;
        let refresh_exp = now + REFRESH_TOKEN_TTL_SECONDS;

        let access_token = self
            .sign_token(user_id, now, access_exp, "access")
            .map_err(|_| AuthServiceError::TokenCreation)?;

        let refresh_token = self
            .sign_token(user_id, now, refresh_exp, "refresh")
            .map_err(|_| AuthServiceError::TokenCreation)?;

        Ok(TokenPair {
            access_token,
            refresh_token,
            access_token_expires_at: access_exp,
            refresh_token_expires_at: refresh_exp,
        })
    }

    fn sign_token(
        &self,
        user_id: &str,
        iat: i64,
        exp: i64,
        token_type: &str,
    ) -> Result<String, jsonwebtoken::errors::Error> {
        let claims = Claims {
            sub: user_id.to_string(),
            exp: exp as usize,
            iat: iat as usize,
            token_type: token_type.to_string(),
        };

        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.jwt_secret.as_bytes()),
        )
    }
}

fn map_auth_error(err: AuthError) -> AuthServiceError {
    match err {
        AuthError::InvalidCredentials => AuthServiceError::InvalidCredentials,
        AuthError::AlreadyExists(msg) => AuthServiceError::AlreadyExists(msg),
        AuthError::Internal(msg) => AuthServiceError::Internal(msg),
    }
}

fn current_unix_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after UNIX_EPOCH")
        .as_secs() as i64
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_domain::{AuthError, AuthPort, AuthResult};
    use jsonwebtoken::{EncodingKey, Header, encode};

    struct MockAuthPort {
        auth_result: Result<AuthResult, AuthError>,
        create_result: Result<AuthResult, AuthError>,
    }

    #[async_trait::async_trait]
    impl AuthPort for MockAuthPort {
        async fn authenticate(
            &self,
            _email: &str,
            _password: &str,
        ) -> Result<AuthResult, AuthError> {
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
}
