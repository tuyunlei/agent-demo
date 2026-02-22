use std::sync::Arc;

use agent_app::AuthService;
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
mod tests {
    use super::*;
    use agent_domain::{AuthError, AuthPort, AuthResult};
    use tonic::metadata::MetadataValue;

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
            _email: &str,
            _password: &str,
            _display_name: &str,
        ) -> Result<AuthResult, AuthError> {
            Err(AuthError::Internal("not used".to_string()))
        }
    }

    fn test_auth_service() -> Arc<AuthService> {
        Arc::new(AuthService::new(
            Arc::new(MockAuthPort),
            "test-secret".to_string(),
        ))
    }

    #[test]
    fn interceptor_without_token_returns_unauthenticated() {
        let interceptor = auth_interceptor(test_auth_service());
        let request = Request::new(());

        let err = interceptor(request).unwrap_err();
        assert_eq!(err.code(), tonic::Code::Unauthenticated);
    }

    #[test]
    fn interceptor_with_invalid_token_returns_unauthenticated() {
        let interceptor = auth_interceptor(test_auth_service());
        let mut request = Request::new(());
        request.metadata_mut().insert(
            "authorization",
            MetadataValue::try_from("Bearer invalid-token").unwrap(),
        );

        let err = interceptor(request).unwrap_err();
        assert_eq!(err.code(), tonic::Code::Unauthenticated);
    }

    #[tokio::test]
    async fn interceptor_with_valid_token_injects_user_id() {
        let auth_service = test_auth_service();
        let login_result = auth_service
            .login("test@example.com", "password123")
            .await
            .unwrap();
        let interceptor = auth_interceptor(auth_service);

        let mut request = Request::new(());
        let header_value = format!("Bearer {}", login_result.token_pair.access_token);
        request.metadata_mut().insert(
            "authorization",
            MetadataValue::try_from(header_value).unwrap(),
        );

        let request = interceptor(request).unwrap();
        let user_id = request.extensions().get::<UserId>().unwrap();
        assert_eq!(user_id.0, "user-001");
    }
}
