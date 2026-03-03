use chrono::{DateTime, Utc};

use crate::error::ContextError;

pub trait PromptSection: Send + Sync {
    fn name(&self) -> &'static str;
    fn build(&self, ctx: &PromptSectionContext) -> Result<String, ContextError>;

    fn is_stable(&self) -> bool {
        true
    }

    fn order(&self) -> u16 {
        100
    }

    fn enabled(&self, _ctx: &PromptSectionContext) -> bool {
        true
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromptSectionContext {
    pub tenant_id: String,
    pub user_id: String,
    pub agent_id: String,
    pub session_id: String,
    pub timezone: String,
    pub tools: Vec<ToolPromptSpec>,
    pub now: DateTime<Utc>,
    pub runtime_info: RuntimeInfo,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolPromptSpec {
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeInfo {
    pub model: String,
    pub os: String,
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use super::*;

    struct DemoSection;

    impl PromptSection for DemoSection {
        fn name(&self) -> &'static str {
            "demo"
        }

        fn build(&self, _ctx: &PromptSectionContext) -> Result<String, ContextError> {
            Ok("ok".to_string())
        }
    }

    #[test]
    fn prompt_section_default_methods() {
        let section = DemoSection;
        let ctx = PromptSectionContext {
            tenant_id: "t1".to_string(),
            user_id: "u1".to_string(),
            agent_id: "a1".to_string(),
            session_id: "s1".to_string(),
            timezone: "UTC".to_string(),
            tools: vec![ToolPromptSpec {
                name: "web_search".to_string(),
                description: "search the web".to_string(),
            }],
            now: Utc::now(),
            runtime_info: RuntimeInfo {
                model: "m1".to_string(),
                os: "linux".to_string(),
            },
        };

        assert_eq!(section.name(), "demo");
        assert_eq!(section.order(), 100);
        assert!(section.is_stable());
        assert!(section.enabled(&ctx));
        assert_eq!(section.build(&ctx).expect("build"), "ok");
    }
}
