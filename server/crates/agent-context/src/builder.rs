use chrono::Utc;

use crate::{
    PromptSection,
    composer::SystemPromptComposer,
    error::ContextError,
    section::{PromptSectionContext, RuntimeInfo, ToolPromptSpec},
    sections::{DateTimeSection, IdentitySection, RuntimeSection, SafetySection, ToolsSection},
};

// ---------------------------------------------------------------------------
// ContextBuilder trait (aligned with design doc §3.2)
// ---------------------------------------------------------------------------

/// Core trait: transforms event stream + config into LLM input.
pub trait ContextBuilder: Send + Sync {
    /// Build system prompt only (cacheable across turns).
    fn build_system_prompt(&self, ctx: &PromptSectionContext) -> Result<String, ContextError>;

    /// Select and map history events into model messages.
    fn build_messages(&self, input: &ContextInput) -> Result<Vec<ModelMessage>, ContextError>;

    /// Convenience: build complete context in one call.
    fn build(&self, input: &ContextInput) -> Result<ContextOutput, ContextError> {
        let system_prompt = self.build_system_prompt(&input.prompt_ctx)?;
        let messages = self.build_messages(input)?;
        Ok(ContextOutput {
            system_prompt,
            messages,
            diagnostics: ContextDiagnostics::default(),
        })
    }
}

// ---------------------------------------------------------------------------
// ContextInput (design doc §3.3)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct ContextInput {
    /// PromptSection build context.
    pub prompt_ctx: PromptSectionContext,

    /// Current session event stream (ascending by sequence_number).
    pub events: Vec<agent_domain::events::EventEnvelope>,

    /// Current user message (may also be in events).
    pub current_user_message: Option<ModelMessage>,

    /// Token budget constraints.
    pub token_budget: TokenBudget,

    /// History window policy.
    pub history_policy: HistoryPolicy,
}

// ---------------------------------------------------------------------------
// ContextOutput (design doc §3.4)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct ContextOutput {
    pub system_prompt: String,
    pub messages: Vec<ModelMessage>,
    pub diagnostics: ContextDiagnostics,
}

#[derive(Debug, Clone, Default)]
pub struct ContextDiagnostics {
    pub used_summary_event_id: Option<String>,
    pub dropped_event_ids: Vec<String>,
    pub estimated_prompt_tokens: Option<u32>,
    pub estimated_history_tokens: Option<u32>,
    pub budget_exceeded: bool,
    pub rebuild_system_prompt: bool,
}

// ---------------------------------------------------------------------------
// ModelMessage — LLM-facing message (design doc §3.4 references)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelMessage {
    pub role: String,
    pub content: String,
}

// ---------------------------------------------------------------------------
// TokenBudget (design doc §3.5)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct TokenBudget {
    pub model_context_limit: u32,
    pub reserved_for_completion: u32,
    pub reserved_for_tool_loop: u32,
    pub reserved_for_system_prompt: u32,
}

impl Default for TokenBudget {
    fn default() -> Self {
        Self {
            model_context_limit: 128_000,
            reserved_for_completion: 4_096,
            reserved_for_tool_loop: 0,
            reserved_for_system_prompt: 8_000,
        }
    }
}

// ---------------------------------------------------------------------------
// HistoryPolicy (design doc §3.3 reference)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum HistoryPolicy {
    /// Keep last N events (MVP default).
    KeepRecentN(usize),
}

impl Default for HistoryPolicy {
    fn default() -> Self {
        Self::KeepRecentN(50)
    }
}

// ---------------------------------------------------------------------------
// DefaultContextBuilder
// ---------------------------------------------------------------------------

pub struct DefaultContextBuilder {
    composer: SystemPromptComposer,
}

impl DefaultContextBuilder {
    #[must_use]
    pub fn new(sections: Vec<Box<dyn PromptSection>>) -> Self {
        Self {
            composer: SystemPromptComposer::new(sections),
        }
    }

    #[must_use]
    pub fn with_default_sections() -> Self {
        Self::new(vec![
            Box::new(IdentitySection),
            Box::new(SafetySection),
            Box::new(ToolsSection),
            Box::new(RuntimeSection),
            Box::new(DateTimeSection),
        ])
    }

    /// Helper to create a minimal PromptSectionContext for testing/MVP.
    pub fn mvp_prompt_ctx(
        tenant_id: impl Into<String>,
        user_id: impl Into<String>,
        agent_id: impl Into<String>,
        session_id: impl Into<String>,
        timezone: impl Into<String>,
        tools: Vec<ToolPromptSpec>,
        runtime_info: RuntimeInfo,
    ) -> PromptSectionContext {
        PromptSectionContext {
            tenant_id: tenant_id.into(),
            user_id: user_id.into(),
            agent_id: agent_id.into(),
            session_id: session_id.into(),
            timezone: timezone.into(),
            tools,
            now: Utc::now(),
            runtime_info,
        }
    }
}

impl ContextBuilder for DefaultContextBuilder {
    fn build_system_prompt(&self, ctx: &PromptSectionContext) -> Result<String, ContextError> {
        self.composer.compose(ctx)
    }

    fn build_messages(&self, input: &ContextInput) -> Result<Vec<ModelMessage>, ContextError> {
        // MVP: convert recent events to ChatMessage-like ModelMessages.
        // For now, return current_user_message if present, else empty.
        // Full event→message mapping comes in R-06 (TurnExecutor).
        let mut msgs = Vec::new();
        if let Some(ref msg) = input.current_user_message {
            msgs.push(msg.clone());
        }
        Ok(msgs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_prompt_ctx() -> PromptSectionContext {
        DefaultContextBuilder::mvp_prompt_ctx(
            "tenant",
            "user",
            "agent-xyz",
            "session-1",
            "UTC",
            vec![
                ToolPromptSpec {
                    name: "search".to_string(),
                    description: "search the web".to_string(),
                },
                ToolPromptSpec {
                    name: "calc".to_string(),
                    description: "run calculations".to_string(),
                },
            ],
            RuntimeInfo {
                model: "claude".to_string(),
                os: "linux".to_string(),
            },
        )
    }

    fn test_input() -> ContextInput {
        ContextInput {
            prompt_ctx: test_prompt_ctx(),
            events: Vec::new(),
            current_user_message: Some(ModelMessage {
                role: "user".to_string(),
                content: "hello".to_string(),
            }),
            token_budget: TokenBudget::default(),
            history_policy: HistoryPolicy::default(),
        }
    }

    #[test]
    fn build_system_prompt_contains_sections() {
        let builder = DefaultContextBuilder::with_default_sections();
        let ctx = test_prompt_ctx();

        let prompt = builder.build_system_prompt(&ctx).unwrap();

        assert!(prompt.contains("helpful AI assistant"));
        assert!(prompt.contains("When the user asks about current events"));
        assert!(prompt.contains("search"));
        assert!(prompt.contains("Current time:"));
    }

    #[test]
    fn build_messages_returns_current_message() {
        let builder = DefaultContextBuilder::with_default_sections();
        let input = test_input();

        let msgs = builder.build_messages(&input).unwrap();

        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].content, "hello");
    }

    #[test]
    fn build_messages_empty_when_no_current() {
        let builder = DefaultContextBuilder::with_default_sections();
        let mut input = test_input();
        input.current_user_message = None;

        let msgs = builder.build_messages(&input).unwrap();

        assert!(msgs.is_empty());
    }

    #[test]
    fn build_returns_complete_output() {
        let builder = DefaultContextBuilder::with_default_sections();
        let input = test_input();

        let output = builder.build(&input).unwrap();

        assert!(!output.system_prompt.is_empty());
        assert_eq!(output.messages.len(), 1);
        assert!(!output.diagnostics.budget_exceeded);
    }

    #[test]
    fn build_system_prompt_contains_runtime_info() {
        let builder = DefaultContextBuilder::with_default_sections();
        let ctx = test_prompt_ctx();

        let prompt = builder.build_system_prompt(&ctx).unwrap();

        assert!(prompt.contains("same language"));
    }
}
