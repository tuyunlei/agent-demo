use crate::{TurnError, TurnInput, TurnOutput};

#[async_trait::async_trait]
pub trait ChatRuntime: Send + Sync {
    async fn run_turn(&self, input: TurnInput) -> Result<TurnOutput, TurnError>;
}
