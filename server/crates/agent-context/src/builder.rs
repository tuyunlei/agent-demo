use async_trait::async_trait;
use chrono::Utc;

use crate::{
    PromptSection,
    composer::SystemPromptComposer,
    error::ContextError,
    section::{PromptSectionContext, RuntimeInfo},
    sections::{DateTimeSection, IdentitySection, RuntimeSection, SafetySection, ToolsSection},
};

#[async_trait]
pub trait ContextBuilder: Send + Sync {
    async fn build_context(
        &self,
        session_id: &str,
        current_message: &str,
        config: &ContextBuilderConfig,
    ) -> Result<BuiltContext, ContextError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextBuilderConfig {
    pub max_history_tokens: u32,
    pub max_system_prompt_tokens: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuiltContext {
    pub system_prompt: String,
    pub messages: Vec<agent_domain::ports::ChatMessage>,
}

pub struct DefaultContextBuilder {
    composer: SystemPromptComposer,
    base_context: PromptSectionContext,
}

impl DefaultContextBuilder {
    pub fn new(base_context: PromptSectionContext, sections: Vec<Box<dyn PromptSection>>) -> Self {
        Self {
            composer: SystemPromptComposer::new(sections),
            base_context,
        }
    }

    pub fn with_default_sections(base_context: PromptSectionContext) -> Self {
        Self::new(
            base_context,
            vec![
                Box::new(IdentitySection),
                Box::new(SafetySection),
                Box::new(ToolsSection),
                Box::new(RuntimeSection),
                Box::new(DateTimeSection),
            ],
        )
    }

    pub fn mvp_context(
        tenant_id: impl Into<String>,
        user_id: impl Into<String>,
        agent_id: impl Into<String>,
        session_id: impl Into<String>,
        tool_names: Vec<String>,
        model: impl Into<String>,
        os: impl Into<String>,
    ) -> PromptSectionContext {
        PromptSectionContext {
            tenant_id: tenant_id.into(),
            user_id: user_id.into(),
            agent_id: agent_id.into(),
            session_id: session_id.into(),
            tool_names,
            now: Utc::now(),
            runtime_info: RuntimeInfo {
                model: model.into(),
                os: os.into(),
            },
        }
    }
}

#[async_trait]
impl ContextBuilder for DefaultContextBuilder {
    async fn build_context(
        &self,
        session_id: &str,
        _current_message: &str,
        _config: &ContextBuilderConfig,
    ) -> Result<BuiltContext, ContextError> {
        let mut section_ctx = self.base_context.clone();
        section_ctx.session_id = session_id.to_string();
        section_ctx.now = Utc::now();

        let system_prompt = self.composer.compose(&section_ctx)?;

        Ok(BuiltContext {
            system_prompt,
            messages: Vec::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use futures::executor::block_on;

    use super::{ContextBuilder, ContextBuilderConfig, DefaultContextBuilder};

    #[test]
    fn build_context_returns_system_prompt_and_empty_messages() {
        let base_context = DefaultContextBuilder::mvp_context(
            "tenant",
            "user",
            "agent-1",
            "session-0",
            vec!["search".to_string(), "calc".to_string()],
            "claude",
            "linux",
        );
        let builder = DefaultContextBuilder::with_default_sections(base_context);

        let built = block_on(builder.build_context(
            "session-1",
            "hello",
            &ContextBuilderConfig {
                max_history_tokens: 1024,
                max_system_prompt_tokens: 2048,
            },
        ))
        .expect("build_context should succeed");

        assert!(built.messages.is_empty());
        assert!(!built.system_prompt.is_empty());
    }

    #[test]
    fn system_prompt_contains_builtin_sections_output() {
        let base_context = DefaultContextBuilder::mvp_context(
            "tenant",
            "user",
            "agent-xyz",
            "session-0",
            vec!["read".to_string()],
            "gpt-test",
            "ubuntu",
        );
        let builder = DefaultContextBuilder::with_default_sections(base_context);

        let built = block_on(builder.build_context(
            "session-2",
            "ping",
            &ContextBuilderConfig {
                max_history_tokens: 256,
                max_system_prompt_tokens: 1024,
            },
        ))
        .expect("build_context should succeed");

        assert!(built.system_prompt.contains("agent-xyz"));
        assert!(built.system_prompt.contains("Respect safety constraints"));
        assert!(built.system_prompt.contains("Available tools: read"));
        assert!(
            built
                .system_prompt
                .contains("Runtime info: model=gpt-test, os=ubuntu")
        );
        assert!(built.system_prompt.contains("Current UTC time:"));
    }
}
