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
    pub tool_names: Vec<String>,
    pub now: DateTime<Utc>,
    pub runtime_info: RuntimeInfo,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeInfo {
    pub model: String,
    pub os: String,
}
