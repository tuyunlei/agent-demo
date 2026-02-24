use agent_orchestrator::AuthServiceError;
use agent_domain::{AgentError, LlmError, StoreError};
use tonic::Status;

pub trait IntoGrpcStatus {
    fn into_status(self) -> Status;
}

pub fn into_status<E>(err: E) -> Status
where
    E: IntoGrpcStatus,
{
    err.into_status()
}

impl IntoGrpcStatus for AuthServiceError {
    fn into_status(self) -> Status {
        match self {
            AuthServiceError::InvalidCredentials => Status::unauthenticated("invalid credentials"),
            AuthServiceError::AlreadyExists(msg) => Status::already_exists(msg),
            AuthServiceError::InvalidInput(msg) => Status::invalid_argument(msg),
            AuthServiceError::TokenCreation | AuthServiceError::TokenValidation => {
                Status::internal("authentication error")
            }
            AuthServiceError::Internal(msg) => Status::internal(msg),
        }
    }
}

impl IntoGrpcStatus for AgentError {
    fn into_status(self) -> Status {
        match self {
            AgentError::InvalidInput(msg) => Status::invalid_argument(msg),
            AgentError::Llm(llm_err) => llm_err.into_status(),
            AgentError::Store(store_err) => store_err.into_status(),
        }
    }
}

impl IntoGrpcStatus for LlmError {
    fn into_status(self) -> Status {
        match self {
            LlmError::RateLimited => Status::resource_exhausted("rate limited"),
            LlmError::Timeout => Status::deadline_exceeded("request timeout"),
            LlmError::ProviderError(_) => Status::internal("AI service error"),
            LlmError::InvalidRequest(msg) => Status::invalid_argument(msg),
        }
    }
}

impl IntoGrpcStatus for StoreError {
    fn into_status(self) -> Status {
        match self {
            StoreError::NotFound(msg) => Status::not_found(msg),
            StoreError::Internal(msg) => Status::internal(msg),
        }
    }
}

#[cfg(test)]
mod tests {
    use agent_domain::{AgentError, LlmError, StoreError};
    use tonic::Code;

    use super::*;

    #[test]
    fn auth_errors_map_to_expected_status() {
        assert_status(
            AuthServiceError::InvalidCredentials,
            Code::Unauthenticated,
            "invalid credentials",
        );
        assert_status(
            AuthServiceError::AlreadyExists("email already in use".to_string()),
            Code::AlreadyExists,
            "email already in use",
        );
        assert_status(
            AuthServiceError::InvalidInput("bad input".to_string()),
            Code::InvalidArgument,
            "bad input",
        );
        assert_status(
            AuthServiceError::TokenCreation,
            Code::Internal,
            "authentication error",
        );
        assert_status(
            AuthServiceError::TokenValidation,
            Code::Internal,
            "authentication error",
        );
        assert_status(
            AuthServiceError::Internal("boom".to_string()),
            Code::Internal,
            "boom",
        );
    }

    #[test]
    fn llm_errors_map_to_expected_status() {
        assert_status(
            LlmError::RateLimited,
            Code::ResourceExhausted,
            "rate limited",
        );
        assert_status(LlmError::Timeout, Code::DeadlineExceeded, "request timeout");
        assert_status(
            LlmError::ProviderError("provider down".to_string()),
            Code::Internal,
            "AI service error",
        );
        assert_status(
            LlmError::InvalidRequest("invalid prompt".to_string()),
            Code::InvalidArgument,
            "invalid prompt",
        );
    }

    #[test]
    fn store_errors_map_to_expected_status() {
        assert_status(
            StoreError::NotFound("session not found".to_string()),
            Code::NotFound,
            "session not found",
        );
        assert_status(
            StoreError::Internal("db error".to_string()),
            Code::Internal,
            "db error",
        );
    }

    #[test]
    fn agent_errors_map_to_expected_status() {
        assert_status(
            AgentError::InvalidInput("bad request".to_string()),
            Code::InvalidArgument,
            "bad request",
        );
        assert_status(
            AgentError::Llm(LlmError::RateLimited),
            Code::ResourceExhausted,
            "rate limited",
        );
        assert_status(
            AgentError::Llm(LlmError::Timeout),
            Code::DeadlineExceeded,
            "request timeout",
        );
        assert_status(
            AgentError::Llm(LlmError::ProviderError("provider down".to_string())),
            Code::Internal,
            "AI service error",
        );
        assert_status(
            AgentError::Llm(LlmError::InvalidRequest("invalid tool input".to_string())),
            Code::InvalidArgument,
            "invalid tool input",
        );
        assert_status(
            AgentError::Store(StoreError::NotFound("session not found".to_string())),
            Code::NotFound,
            "session not found",
        );
        assert_status(
            AgentError::Store(StoreError::Internal("db error".to_string())),
            Code::Internal,
            "db error",
        );
    }

    fn assert_status<E>(err: E, expected_code: Code, expected_message: &str)
    where
        E: IntoGrpcStatus,
    {
        let status = err.into_status();
        assert_eq!(status.code(), expected_code);
        assert_eq!(status.message(), expected_message);
    }
}
