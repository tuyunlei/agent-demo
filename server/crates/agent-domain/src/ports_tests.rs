use super::{
    AppendResult, AuthError, CreateSessionParams, EventRange, EventStoreError, NewEvent,
    SessionListFilter,
};

#[test]
fn auth_error_variants() {
    let invalid = AuthError::InvalidCredentials;
    let exists = AuthError::AlreadyExists("email already in use".to_string());
    let internal = AuthError::Internal("db down".to_string());

    assert_eq!(invalid, AuthError::InvalidCredentials);
    assert_eq!(
        exists,
        AuthError::AlreadyExists("email already in use".to_string())
    );
    assert_eq!(internal, AuthError::Internal("db down".to_string()));
    assert!(format!("{internal:?}").contains("db down"));
}

#[test]
fn event_store_error_display_messages() {
    assert_eq!(
        EventStoreError::SessionNotFound("s1".to_string()).to_string(),
        "session not found: s1"
    );
    assert_eq!(
        EventStoreError::SessionAlreadyExists("s1".to_string()).to_string(),
        "session already exists: s1"
    );
    assert_eq!(
        EventStoreError::SequenceConflict.to_string(),
        "sequence conflict"
    );
    assert_eq!(
        EventStoreError::Database("db down".to_string()).to_string(),
        "database error: db down"
    );
}

#[test]
fn event_range_defaults() {
    let range = EventRange::default();
    assert_eq!(range.start_inclusive, None);
    assert_eq!(range.end_inclusive, None);
}

#[test]
fn event_and_append_structs_construct() {
    let event = NewEvent {
        event_id: "evt-1".to_string(),
        event_type: "UserMessage".to_string(),
        payload: serde_json::json!({"k": "v"}),
        tenant_id: "t1".to_string(),
        user_id: "u1".to_string(),
    };
    assert_eq!(event.event_type, "UserMessage");
    assert_eq!(event.payload["k"], "v");

    let append = AppendResult {
        last_sequence: 42,
        session_event_count: 7,
    };
    assert_eq!(append.last_sequence, 42);
    assert_eq!(append.session_event_count, 7);
}

#[test]
fn session_params_and_filter_structs_construct() {
    let params = CreateSessionParams {
        tenant_id: "t1".to_string(),
        user_id: "u1".to_string(),
        agent_id: "a1".to_string(),
        title: Some("hello".to_string()),
    };
    assert_eq!(params.agent_id, "a1");
    assert_eq!(params.title.as_deref(), Some("hello"));

    let filter = SessionListFilter {
        user_id: "u1".to_string(),
        tenant_id: "t1".to_string(),
        include_archived: true,
        limit: 10,
        offset: 20,
    };
    assert!(filter.include_archived);
    assert_eq!(filter.limit, 10);
    assert_eq!(filter.offset, 20);
}
