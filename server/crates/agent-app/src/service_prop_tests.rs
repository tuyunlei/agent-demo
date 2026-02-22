use std::sync::Arc;

use super::*;
use agent_domain::{AuthError, AuthPort, AuthResult};
use proptest::prelude::*;

struct DummyAuthPort;

#[async_trait::async_trait]
impl AuthPort for DummyAuthPort {
    async fn authenticate(&self, _: &str, _: &str) -> Result<AuthResult, AuthError> {
        Err(AuthError::Internal(
            "dummy auth port should not be called in property tests".to_string(),
        ))
    }

    async fn create_user(&self, _: &str, _: &str, _: &str) -> Result<AuthResult, AuthError> {
        Err(AuthError::Internal(
            "dummy auth port should not be called in property tests".to_string(),
        ))
    }
}

fn test_service(secret: String) -> AuthService {
    AuthService::new(Arc::new(DummyAuthPort), secret)
}

proptest! {
    #[test]
    fn jwt_roundtrip_any_user_id(user_id in "[a-zA-Z0-9_-]{1,64}") {
        let service = test_service("test-secret-key".to_string());

        let token_pair = service.create_token_pair(&user_id).expect("token pair should be generated");
        let claims = service
            .validate_token(&token_pair.access_token)
            .expect("signed access token should validate");

        prop_assert_eq!(claims.sub, user_id);
        prop_assert_eq!(claims.token_type, "access");
    }

    #[test]
    fn jwt_different_secrets_reject(
        secret_a in "[a-zA-Z0-9]{8,32}",
        secret_b in "[a-zA-Z0-9]{8,32}",
    ) {
        prop_assume!(secret_a != secret_b);

        let service_a = test_service(secret_a);
        let service_b = test_service(secret_b);

        let token_pair = service_a
            .create_token_pair("user-1")
            .expect("token pair should be generated");

        let result = service_b.validate_token(&token_pair.access_token);
        prop_assert!(result.is_err());
    }

    #[test]
    fn access_expires_before_refresh(user_id in "[a-zA-Z0-9]{1,32}") {
        let service = test_service("secret".to_string());

        let token_pair = service
            .create_token_pair(&user_id)
            .expect("token pair should be generated");

        prop_assert!(token_pair.access_token_expires_at < token_pair.refresh_token_expires_at);
    }
}
