use std::sync::Arc;

use agent_app::AuthService;
use agent_proto::auth_service_server::AuthService as AuthServiceTrait;
use agent_proto::{
    LoginRequest, LoginResponse, LogoutRequest, LogoutResponse, RefreshTokenRequest,
    RefreshTokenResponse, RegisterRequest, RegisterResponse, TokenPair,
};
use prost_types::Timestamp;
use tonic::{Request, Response, Status};

use super::error::into_status;

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
        request: Request<RegisterRequest>,
    ) -> Result<Response<RegisterResponse>, Status> {
        let req = request.into_inner();
        let result = self
            .auth_service
            .register(&req.email, &req.password, &req.display_name)
            .await
            .map_err(into_status)?;

        Ok(Response::new(RegisterResponse {
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

    async fn login(
        &self,
        request: Request<LoginRequest>,
    ) -> Result<Response<LoginResponse>, Status> {
        let req = request.into_inner();

        let result = self
            .auth_service
            .login(&req.email, &req.password)
            .await
            .map_err(into_status)?;

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
        request: Request<RefreshTokenRequest>,
    ) -> Result<Response<RefreshTokenResponse>, Status> {
        let req = request.into_inner();

        let result = self
            .auth_service
            .refresh_token(&req.refresh_token)
            .await
            .map_err(into_status)?;

        Ok(Response::new(RefreshTokenResponse {
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

    async fn logout(
        &self,
        _request: Request<LogoutRequest>,
    ) -> Result<Response<LogoutResponse>, Status> {
        Err(Status::unimplemented("not implemented"))
    }
}

fn to_timestamp(seconds: i64) -> Timestamp {
    Timestamp { seconds, nanos: 0 }
}

#[cfg(test)]
#[path = "auth_handler_tests.rs"]
mod tests;
