use std::sync::Arc;

use agent_orchestrator::AuthService;
use tonic::{Request, Status};

#[derive(Debug, Clone)]
pub struct UserId(pub String);

pub fn auth_interceptor(
    auth_service: Arc<AuthService>,
) -> impl Fn(Request<()>) -> Result<Request<()>, Status> + Clone {
    move |mut request: Request<()>| {
        let auth_header = request
            .metadata()
            .get("authorization")
            .and_then(|value| value.to_str().ok())
            .ok_or_else(|| Status::unauthenticated("invalid or expired token"))?;

        let token = auth_header
            .strip_prefix("Bearer ")
            .ok_or_else(|| Status::unauthenticated("invalid or expired token"))?;

        let claims = auth_service
            .validate_token(token)
            .map_err(|_| Status::unauthenticated("invalid or expired token"))?;

        request.extensions_mut().insert(UserId(claims.sub));
        Ok(request)
    }
}

#[cfg(test)]
#[path = "auth_interceptor_tests.rs"]
mod tests;
