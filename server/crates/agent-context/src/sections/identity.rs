use crate::{PromptSection, error::ContextError, section::PromptSectionContext};

pub struct IdentitySection;

impl PromptSection for IdentitySection {
    fn name(&self) -> &'static str {
        "identity"
    }

    fn build(&self, ctx: &PromptSectionContext) -> Result<String, ContextError> {
        Ok(format!(
            "You are agent '{}'. Follow your role boundaries, stay helpful, and prioritize user intent.",
            ctx.agent_id
        ))
    }

    fn order(&self) -> u16 {
        10
    }
}
