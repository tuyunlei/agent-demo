use crate::{PromptSection, error::ContextError, section::PromptSectionContext};

pub struct ToolsSection;

impl PromptSection for ToolsSection {
    fn name(&self) -> &'static str {
        "tools"
    }

    fn build(&self, ctx: &PromptSectionContext) -> Result<String, ContextError> {
        if ctx.tool_names.is_empty() {
            return Ok("Available tools: (none)".to_string());
        }

        Ok(format!("Available tools: {}", ctx.tool_names.join(", ")))
    }

    fn order(&self) -> u16 {
        30
    }
}
