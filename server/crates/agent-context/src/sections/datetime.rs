use crate::{PromptSection, error::ContextError, section::PromptSectionContext};

pub struct DateTimeSection;

impl PromptSection for DateTimeSection {
    fn name(&self) -> &'static str {
        "datetime"
    }

    fn build(&self, ctx: &PromptSectionContext) -> Result<String, ContextError> {
        Ok(format!("Current UTC time: {}", ctx.now.to_rfc3339()))
    }

    fn is_stable(&self) -> bool {
        false
    }

    fn order(&self) -> u16 {
        100
    }
}
