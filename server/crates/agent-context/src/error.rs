#[derive(Debug, thiserror::Error)]
pub enum ContextError {
    #[error("failed to build section '{section}': {message}")]
    SectionBuildFailed {
        section: &'static str,
        message: String,
    },

    #[error("invalid context: {0}")]
    InvalidContext(String),
}
