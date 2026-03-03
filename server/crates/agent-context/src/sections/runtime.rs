use crate::{PromptSection, error::ContextError, section::PromptSectionContext};

pub struct RuntimeSection;

impl PromptSection for RuntimeSection {
    fn name(&self) -> &'static str {
        "runtime"
    }

    fn build(&self, _ctx: &PromptSectionContext) -> Result<String, ContextError> {
        Ok("Be concise and helpful. Respond in the same language the user uses.".to_string())
    }

    fn order(&self) -> u16 {
        90
    }
}
