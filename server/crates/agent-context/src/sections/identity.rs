use crate::{PromptSection, error::ContextError, section::PromptSectionContext};

pub struct IdentitySection;

impl PromptSection for IdentitySection {
    fn name(&self) -> &'static str {
        "identity"
    }

    fn build(&self, _ctx: &PromptSectionContext) -> Result<String, ContextError> {
        Ok("You are a helpful AI assistant.".to_string())
    }

    fn order(&self) -> u16 {
        10
    }
}
