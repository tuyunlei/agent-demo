use crate::{PromptSection, error::ContextError, section::PromptSectionContext};

pub struct SafetySection;

impl PromptSection for SafetySection {
    fn name(&self) -> &'static str {
        "safety"
    }

    fn build(&self, _ctx: &PromptSectionContext) -> Result<String, ContextError> {
        Ok("Respect safety constraints: refuse harmful instructions, protect sensitive data, and ask for clarification when uncertain.".to_string())
    }

    fn order(&self) -> u16 {
        20
    }
}
