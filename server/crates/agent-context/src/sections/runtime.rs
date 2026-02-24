use crate::{PromptSection, error::ContextError, section::PromptSectionContext};

pub struct RuntimeSection;

impl PromptSection for RuntimeSection {
    fn name(&self) -> &'static str {
        "runtime"
    }

    fn build(&self, ctx: &PromptSectionContext) -> Result<String, ContextError> {
        Ok(format!(
            "Runtime info: model={}, os={}",
            ctx.runtime_info.model, ctx.runtime_info.os
        ))
    }

    fn order(&self) -> u16 {
        90
    }
}
