use crate::{PromptSection, error::ContextError, section::PromptSectionContext};

pub struct DateTimeSection;

impl PromptSection for DateTimeSection {
    fn name(&self) -> &'static str {
        "datetime"
    }

    fn build(&self, ctx: &PromptSectionContext) -> Result<String, ContextError> {
        let timezone = ctx
            .timezone
            .parse::<chrono_tz::Tz>()
            .unwrap_or(chrono_tz::UTC);
        let now = ctx
            .now
            .with_timezone(&timezone)
            .format("%Y-%m-%d %H:%M")
            .to_string();
        Ok(format!("Current time: {now} ({})", ctx.timezone))
    }

    fn is_stable(&self) -> bool {
        false
    }

    fn order(&self) -> u16 {
        100
    }
}
