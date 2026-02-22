use super::{AgentError, AuthError, LlmError, StoreError};

#[test]
fn auth_error_variants() {
    let invalid = AuthError::InvalidCredentials;
    let exists = AuthError::AlreadyExists("email already in use".to_string());
    let internal = AuthError::Internal("db down".to_string());

    assert_eq!(invalid, AuthError::InvalidCredentials);
    assert_eq!(
        exists,
        AuthError::AlreadyExists("email already in use".to_string())
    );
    assert_eq!(internal, AuthError::Internal("db down".to_string()));
    assert!(format!("{internal:?}").contains("db down"));
}

#[test]
fn llm_error_variants() {
    let provider = LlmError::ProviderError("bad response".to_string());
    let invalid = LlmError::InvalidRequest("missing messages".to_string());

    assert_eq!(LlmError::RateLimited, LlmError::RateLimited);
    assert_eq!(
        provider,
        LlmError::ProviderError("bad response".to_string())
    );
    assert_eq!(
        invalid,
        LlmError::InvalidRequest("missing messages".to_string())
    );
    assert!(format!("{:?}", LlmError::Timeout).contains("Timeout"));
}

#[test]
fn agent_error_from_llm_error() {
    let err = AgentError::from(LlmError::RateLimited);

    assert_eq!(err, AgentError::Llm(LlmError::RateLimited));
}

#[test]
fn agent_error_from_store_error() {
    let err = AgentError::from(StoreError::Internal("db down".to_string()));

    assert_eq!(
        err,
        AgentError::Store(StoreError::Internal("db down".to_string()))
    );
}
