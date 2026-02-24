use agent_domain::StoreError;
use agent_orchestrator::{AuthServiceError, TurnError};
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

impl IntoGrpcStatus for TurnError {
    fn into_status(self) -> Status {
        match self {
            TurnError::InvalidInput(msg) => Status::invalid_argument(msg),
            TurnError::SessionError(msg) => Status::internal(msg),
            TurnError::LlmError(_) => Status::internal("AI service error"),
            TurnError::ToolLoopExceeded { max } => {
                Status::failed_precondition(format!("tool loop exceeded max iterations: {max}"))
            }
            TurnError::Internal(msg) => Status::internal(msg),
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
    use agent_orchestrator::{AuthServiceError, TurnError};
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
    }

    #[test]
    fn turn_errors_map_to_expected_status() {
        assert_status(
            TurnError::InvalidInput("bad request".to_string()),
            Code::InvalidArgument,
            "bad request",
        );
        assert_status(
            TurnError::LlmError("provider down".to_string()),
            Code::Internal,
            "AI service error",
        );
        assert_status(
            TurnError::ToolLoopExceeded { max: 2 },
            Code::FailedPrecondition,
            "tool loop exceeded max iterations: 2",
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
