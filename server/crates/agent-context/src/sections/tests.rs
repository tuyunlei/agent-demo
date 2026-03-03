use chrono::Utc;

use crate::{
    PromptSection, PromptSectionContext, RuntimeInfo, SystemPromptComposer, ToolPromptSpec,
    sections::{DateTimeSection, IdentitySection, RuntimeSection, SafetySection, ToolsSection},
};

fn ctx() -> PromptSectionContext {
    PromptSectionContext {
        tenant_id: "t".to_string(),
        user_id: "u".to_string(),
        agent_id: "agent-a".to_string(),
        session_id: "s".to_string(),
        timezone: "UTC".to_string(),
        tools: vec![
            ToolPromptSpec {
                name: "read".to_string(),
                description: "read from store".to_string(),
            },
            ToolPromptSpec {
                name: "write".to_string(),
                description: "write to store".to_string(),
            },
        ],
        now: Utc::now(),
        runtime_info: RuntimeInfo {
            model: "model-x".to_string(),
            os: "linux".to_string(),
        },
    }
}

#[test]
fn identity_section_builds_non_empty() {
    let out = IdentitySection.build(&ctx()).expect("build should succeed");
    assert!(!out.is_empty());
    assert!(out.contains("helpful AI assistant"));
}

#[test]
fn safety_section_builds_non_empty() {
    let out = SafetySection.build(&ctx()).expect("build should succeed");
    assert!(!out.is_empty());
    assert!(out.contains("current events"));
}

#[test]
fn tools_section_builds_non_empty() {
    let out = ToolsSection.build(&ctx()).expect("build should succeed");
    assert!(!out.is_empty());
    assert!(out.contains("- read: read from store"));
}

#[test]
fn runtime_section_builds_non_empty() {
    let out = RuntimeSection.build(&ctx()).expect("build should succeed");
    assert!(!out.is_empty());
    assert!(out.contains("same language"));
}

#[test]
fn datetime_section_builds_non_empty() {
    let out = DateTimeSection.build(&ctx()).expect("build should succeed");
    assert!(!out.is_empty());
}

#[test]
fn datetime_section_invalid_timezone_falls_back_label_to_utc() {
    let mut invalid_tz_ctx = ctx();
    invalid_tz_ctx.timezone = "Invalid/Zone".to_string();

    let out = DateTimeSection
        .build(&invalid_tz_ctx)
        .expect("build should succeed");

    assert!(out.contains("(UTC)"));
    assert!(!out.contains("(Invalid/Zone)"));
}

#[test]
fn datetime_section_is_not_stable() {
    assert!(!DateTimeSection.is_stable());
}

struct DisabledSection;

impl PromptSection for DisabledSection {
    fn name(&self) -> &'static str {
        "disabled"
    }

    fn build(&self, _ctx: &PromptSectionContext) -> Result<String, crate::ContextError> {
        Ok("should not appear".to_string())
    }

    fn enabled(&self, _ctx: &PromptSectionContext) -> bool {
        false
    }
}

#[test]
fn composer_skips_disabled_section() {
    let composer =
        SystemPromptComposer::new(vec![Box::new(DisabledSection), Box::new(SafetySection)]);
    let out = composer.compose(&ctx()).expect("compose should succeed");
    assert!(!out.contains("should not appear"));
    assert!(out.contains("current events"));
}

#[test]
fn builtin_orders_are_in_expected_sequence() {
    let composer = SystemPromptComposer::new(vec![
        Box::new(DateTimeSection),
        Box::new(ToolsSection),
        Box::new(IdentitySection),
    ]);

    let out = composer.compose(&ctx()).expect("compose should succeed");
    let i_identity = out
        .find("You are a helpful AI assistant.")
        .expect("identity present");
    let i_tools = out
        .find("You have access to the following tools")
        .expect("tools present");
    let i_time = out.find("Current time:").expect("datetime present");

    assert!(i_identity < i_tools);
    assert!(i_tools < i_time);
}
