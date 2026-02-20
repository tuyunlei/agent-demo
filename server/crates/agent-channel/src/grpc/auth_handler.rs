use std::sync::Arc;

use agent_app::{AuthService, AuthServiceError};
use agent_proto::auth_service_server::AuthService as AuthServiceTrait;
use agent_proto::{
    LoginRequest, LoginResponse, LogoutRequest, LogoutResponse, RefreshTokenRequest,
    RefreshTokenResponse, RegisterRequest, RegisterResponse, TokenPair,
};
use prost_types::Timestamp;
use tonic::{Request, Response, Status};

pub struct AuthServiceHandler {
    auth_service: Arc<AuthService>,
}

impl AuthServiceHandler {
    pub fn new(auth_service: Arc<AuthService>) -> Self {
        Self { auth_service }
    }
}

#[tonic::async_trait]
impl AuthServiceTrait for AuthServiceHandler {
    async fn register(
        &self,
        _request: Request<RegisterRequest>,
    ) -> Result<Response<RegisterResponse>, Status> {
        Err(Status::unimplemented("not implemented"))
    }

    async fn login(
        &self,
        request: Request<LoginRequest>,
    ) -> Result<Response<LoginResponse>, Status> {
        let req = request.into_inner();

        let result = self
            .auth_service
            .login(&req.email, &req.password)
            .await
            .map_err(map_error)?;

        Ok(Response::new(LoginResponse {
            user_id: result.user_id,
            token_pair: Some(TokenPair {
                access_token: result.token_pair.access_token,
                refresh_token: result.token_pair.refresh_token,
                access_token_expires_at: Some(to_timestamp(
                    result.token_pair.access_token_expires_at,
                )),
                refresh_token_expires_at: Some(to_timestamp(
                    result.token_pair.refresh_token_expires_at,
                )),
            }),
        }))
    }

    async fn refresh_token(
        &self,
        _request: Request<RefreshTokenRequest>,
    ) -> Result<Response<RefreshTokenResponse>, Status> {
        Err(Status::unimplemented("not implemented"))
    }

    async fn logout(
        &self,
        _request: Request<LogoutRequest>,
    ) -> Result<Response<LogoutResponse>, Status> {
        Err(Status::unimplemented("not implemented"))
    }
}

fn map_error(err: AuthServiceError) -> Status {
    match err {
        AuthServiceError::InvalidCredentials => Status::unauthenticated("invalid credentials"),
        AuthServiceError::TokenCreation | AuthServiceError::TokenValidation => {
            Status::internal("token handling failed")
        }
        AuthServiceError::Internal(msg) => Status::internal(msg),
    }
}

fn to_timestamp(seconds: i64) -> Timestamp {
    Timestamp { seconds, nanos: 0 }
}

#[cfg(test)]
mod tests {
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
}
