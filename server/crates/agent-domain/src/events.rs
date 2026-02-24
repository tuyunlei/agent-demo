#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct EventEnvelope {
    pub meta: EventMeta,
    pub payload: EventPayload,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct EventMeta {
    pub event_id: String,
    pub session_id: String,
    pub sequence_number: u64,
    pub timestamp: i64,
    pub tenant_id: String,
    pub user_id: String,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum EventPayload {
    UserMessage(UserMessageEvent),
    AssistantMessage(AssistantMessageEvent),
    ToolCallRequest(ToolCallRequestEvent),
    ToolCallResult(ToolCallResultEvent),
    SystemEvent(SystemEvent),
    ConfigChange(ConfigChangeEvent),
    CompactionMarker(CompactionMarkerEvent),
    Summary(SummaryEvent),
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct AttachmentRef {
    pub kind: String,
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct UserMessageEvent {
    pub message_id: String,
    pub text: String,
    pub attachments: Vec<AttachmentRef>,
    pub input_channel: String,
    pub client_message_id: Option<String>,
    pub token_estimate: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct AssistantMessageEvent {
    pub message_id: String,
    pub text: String,
    pub finish_reason: AssistantFinishReason,
    pub model: String,
    pub prompt_tokens: Option<u32>,
    pub completion_tokens: Option<u32>,
    pub tool_call_count: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum AssistantFinishReason {
    Stop,
    ToolCalls,
    Length,
    Safety,
    ErrorFallback,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ToolCallRequestEvent {
    pub request_id: String,
    pub provider_call_id: String,
    pub tool_name: String,
    pub arguments_json: String,
    pub timeout_ms: Option<u32>,
    pub attempt: u16,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ToolCallResultEvent {
    pub request_id: String,
    pub result_id: String,
    pub status: ToolCallStatus,
    pub output_json: Option<String>,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub latency_ms: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ToolCallStatus {
    Success,
    Timeout,
    Cancelled,
    Failed,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SystemEvent {
    pub kind: SystemEventKind,
    pub actor: SystemActor,
    pub detail_json: String,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SystemEventKind {
    SessionCreated,
    SessionArchived,
    SessionRestored,
    TurnStarted,
    TurnCompleted,
    TurnFailed,
    PolicyNotice,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SystemActor {
    Orchestrator,
    Lifecycle,
    Scheduler,
    Operator,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ConfigChangeEvent {
    pub change_id: String,
    pub scope: ConfigScope,
    pub changed_fields: Vec<String>,
    pub before_hash: Option<String>,
    pub after_hash: String,
    pub patch_json: String,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ConfigScope {
    Persona,
    Toolset,
    SafetyPolicy,
    RuntimeParam,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CompactionMarkerEvent {
    pub compaction_id: String,
    pub strategy: CompactionStrategy,
    pub replaced_range_start_seq: u64,
    pub replaced_range_end_seq: u64,
    pub summary_event_id: String,
    pub trigger: CompactionTrigger,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum CompactionStrategy {
    SlidingSummary,
    HierarchicalSummary,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum CompactionTrigger {
    TokenThreshold,
    EventCountThreshold,
    Manual,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SummaryEvent {
    pub summary_id: String,
    pub source_range_start_seq: u64,
    pub source_range_end_seq: u64,
    pub summary_text: String,
    pub key_facts: Vec<String>,
    pub open_threads: Vec<String>,
    pub generated_by: SummaryGenerator,
    pub token_count: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SummaryGenerator {
    Llm,
    RuleBased,
    Hybrid,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_envelope_construct_and_serialize() {
        let event = EventEnvelope {
            meta: EventMeta {
                event_id: "evt-1".to_string(),
                session_id: "s-1".to_string(),
                sequence_number: 1,
                timestamp: 1_700_000_000,
                tenant_id: "t-1".to_string(),
                user_id: "u-1".to_string(),
            },
            payload: EventPayload::UserMessage(UserMessageEvent {
                message_id: "m-1".to_string(),
                text: "hello".to_string(),
                attachments: vec![],
                input_channel: "telegram".to_string(),
                client_message_id: None,
                token_estimate: Some(2),
            }),
        };

        let json = serde_json::to_string(&event).expect("serialize event");
        let decoded: EventEnvelope = serde_json::from_str(&json).expect("deserialize event");

        assert_eq!(decoded, event);
    }
}
