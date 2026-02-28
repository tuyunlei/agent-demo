use crate::{PromptSection, error::ContextError, section::PromptSectionContext};

pub struct SystemPromptComposer {
    sections: Vec<Box<dyn PromptSection>>,
}

impl SystemPromptComposer {
    #[must_use]
    pub fn new(sections: Vec<Box<dyn PromptSection>>) -> Self {
        Self { sections }
    }

    pub fn compose(&self, ctx: &PromptSectionContext) -> Result<String, ContextError> {
        let mut ordered: Vec<&dyn PromptSection> = self
            .sections
            .iter()
            .map(Box::as_ref)
            .filter(|section| section.enabled(ctx))
            .collect();

        ordered.sort_by_key(|section| section.order());

        let mut parts = Vec::new();
        for section in ordered {
            let content = section.build(ctx)?;
            let trimmed = content.trim();
            if !trimmed.is_empty() {
                parts.push(trimmed.to_string());
            }
        }

        Ok(parts.join("\n\n"))
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use super::SystemPromptComposer;
    use crate::{ContextError, PromptSection, PromptSectionContext, RuntimeInfo};

    struct TestSection {
        name: &'static str,
        content: &'static str,
        order: u16,
        enabled: bool,
    }

    impl PromptSection for TestSection {
        fn name(&self) -> &'static str {
            self.name
        }

        fn build(&self, _ctx: &PromptSectionContext) -> Result<String, ContextError> {
            Ok(self.content.to_string())
        }

        fn order(&self) -> u16 {
            self.order
        }

        fn enabled(&self, _ctx: &PromptSectionContext) -> bool {
            self.enabled
        }
    }

    fn ctx() -> PromptSectionContext {
        PromptSectionContext {
            tenant_id: "t-1".to_string(),
            user_id: "u-1".to_string(),
            agent_id: "a-1".to_string(),
            session_id: "s-1".to_string(),
            tool_names: vec!["search".to_string()],
            now: Utc::now(),
            runtime_info: RuntimeInfo {
                model: "gpt".to_string(),
                os: "linux".to_string(),
            },
        }
    }

    #[test]
    fn compose_returns_empty_for_empty_sections() {
        let composer = SystemPromptComposer::new(vec![]);
        let out = composer.compose(&ctx()).expect("compose should succeed");
        assert!(out.is_empty());
    }

    #[test]
    fn compose_respects_order() {
        let composer = SystemPromptComposer::new(vec![
            Box::new(TestSection {
                name: "late",
                content: "second",
                order: 20,
                enabled: true,
            }),
            Box::new(TestSection {
                name: "early",
                content: "first",
                order: 10,
                enabled: true,
            }),
        ]);

        let out = composer.compose(&ctx()).expect("compose should succeed");
        assert_eq!(out, "first\n\nsecond");
    }

    #[test]
    fn compose_skips_disabled_sections() {
        let composer = SystemPromptComposer::new(vec![
            Box::new(TestSection {
                name: "off",
                content: "hidden",
                order: 10,
                enabled: false,
            }),
            Box::new(TestSection {
                name: "on",
                content: "shown",
                order: 20,
                enabled: true,
            }),
        ]);

        let out = composer.compose(&ctx()).expect("compose should succeed");
        assert_eq!(out, "shown");
    }

    #[test]
    fn compose_skips_empty_output_sections() {
        let composer = SystemPromptComposer::new(vec![
            Box::new(TestSection {
                name: "empty",
                content: "  ",
                order: 10,
                enabled: true,
            }),
            Box::new(TestSection {
                name: "text",
                content: "value",
                order: 20,
                enabled: true,
            }),
        ]);

        let out = composer.compose(&ctx()).expect("compose should succeed");
        assert_eq!(out, "value");
    }
}
