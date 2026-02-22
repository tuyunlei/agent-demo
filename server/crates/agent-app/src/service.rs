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
#[path = "service_tests.rs"]
mod tests;
