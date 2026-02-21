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

        let now = current_unix_seconds();
        let access_exp = now + ACCESS_TOKEN_TTL_SECONDS;
        let refresh_exp = now + REFRESH_TOKEN_TTL_SECONDS;

        let access_token = self
            .sign_token(&user.user_id, now, access_exp, "access")
            .map_err(|_| AuthServiceError::TokenCreation)?;

        let refresh_token = self
            .sign_token(&user.user_id, now, refresh_exp, "refresh")
            .map_err(|_| AuthServiceError::TokenCreation)?;

        Ok(LoginResult {
            user_id: user.user_id,
            token_pair: TokenPair {
                access_token,
                refresh_token,
                access_token_expires_at: access_exp,
                refresh_token_expires_at: refresh_exp,
            },
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
        should_succeed: bool,
    }

    #[async_trait::async_trait]
    impl AuthPort for MockAuthPort {
        async fn authenticate(
            &self,
            _email: &str,
            _password: &str,
        ) -> Result<AuthResult, AuthError> {
            if self.should_succeed {
                Ok(AuthResult {
                    user_id: "user-001".to_string(),
                    display_name: "Test User".to_string(),
                })
            } else {
                Err(AuthError::InvalidCredentials)
            }
        }
    }

    #[tokio::test]
    async fn login_success_returns_jwt_token_pair() {
        let service = AuthService::new(
            Arc::new(MockAuthPort {
                should_succeed: true,
            }),
            "test-secret".to_string(),
        );

        let result = service
            .login("test@example.com", "password123")
            .await
            .unwrap();

        assert_eq!(result.user_id, "user-001");
        assert!(!result.token_pair.access_token.is_empty());
        assert!(!result.token_pair.refresh_token.is_empty());

        let access_claims = service
            .validate_token(&result.token_pair.access_token)
            .unwrap();
        assert_eq!(access_claims.sub, "user-001");
        assert_eq!(access_claims.token_type, "access");
    }

    #[tokio::test]
    async fn login_failure_returns_invalid_credentials() {
        let service = AuthService::new(
            Arc::new(MockAuthPort {
                should_succeed: false,
            }),
            "test-secret".to_string(),
        );

        let err = service
            .login("wrong@example.com", "wrong-password")
            .await
            .unwrap_err();

        assert_eq!(err, AuthServiceError::InvalidCredentials);
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
        let service = AuthService::new(
            Arc::new(MockAuthPort {
                should_succeed: true,
            }),
            "test-secret".to_string(),
        );
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

    #[test]
    fn validate_token_with_expired_token_fails() {
        let service = AuthService::new(
            Arc::new(MockAuthPort {
                should_succeed: true,
            }),
            "test-secret".to_string(),
        );
        let now = current_unix_seconds() as usize;
        let token = sign_test_token(
            "test-secret",
            &Claims {
                sub: "user-001".to_string(),
                exp: now.saturating_sub(120),
                iat: now.saturating_sub(10),
                token_type: "access".to_string(),
            },
        );

        let result = service.validate_token(&token);

        assert!(matches!(result, Err(AuthServiceError::TokenValidation)));
    }

    #[test]
    fn validate_token_with_wrong_secret_fails() {
        let service = AuthService::new(
            Arc::new(MockAuthPort {
                should_succeed: true,
            }),
            "correct-secret".to_string(),
        );
        let now = current_unix_seconds() as usize;
        let token = sign_test_token(
            "different-secret",
            &Claims {
                sub: "user-001".to_string(),
                exp: now + 3600,
                iat: now,
                token_type: "access".to_string(),
            },
        );

        let result = service.validate_token(&token);

        assert!(matches!(result, Err(AuthServiceError::TokenValidation)));
    }
}
