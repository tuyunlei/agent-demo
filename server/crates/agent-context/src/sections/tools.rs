use crate::{PromptSection, error::ContextError, section::PromptSectionContext};

pub struct ToolsSection;

impl PromptSection for ToolsSection {
    fn name(&self) -> &'static str {
        "tools"
    }

    fn build(&self, ctx: &PromptSectionContext) -> Result<String, ContextError> {
        if ctx.tools.is_empty() {
            return Ok("You have access to the following tools:\n- (none)".to_string());
        }

        let tools = ctx
            .tools
            .iter()
            .map(|tool| format!("- {}: {}", tool.name, tool.description))
            .collect::<Vec<_>>()
            .join("\n");
        Ok(format!("You have access to the following tools:\n{tools}"))
    }

    fn order(&self) -> u16 {
        30
    }
}
