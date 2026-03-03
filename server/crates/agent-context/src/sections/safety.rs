use crate::{PromptSection, error::ContextError, section::PromptSectionContext};

pub struct SafetySection;

impl PromptSection for SafetySection {
    fn name(&self) -> &'static str {
        "safety"
    }

    fn build(&self, _ctx: &PromptSectionContext) -> Result<String, ContextError> {
        Ok("When the user asks about current events, time, or facts you're unsure about, use the appropriate tool.".to_string())
    }

    fn order(&self) -> u16 {
        20
    }
}
