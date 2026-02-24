use agent_domain::EventStoreError;
use agent_domain::{
    events::{EventEnvelope, EventMeta, EventPayload, EventProducer},
    session::{Session, SessionStatus},
};
use sqlx::{Row, postgres::PgRow};
use uuid::Uuid;

pub fn parse_uuid(raw: &str) -> Result<Uuid, EventStoreError> {
    Uuid::parse_str(raw).map_err(|err| EventStoreError::Database(err.to_string()))
}

pub fn u64_to_i64(value: Option<u64>) -> Result<Option<i64>, EventStoreError> {
    value
        .map(|v| i64::try_from(v).map_err(|e| EventStoreError::Database(e.to_string())))
        .transpose()
}

pub fn i64_to_u64(value: i64) -> Result<u64, EventStoreError> {
    u64::try_from(value).map_err(|e| EventStoreError::Database(e.to_string()))
}

pub fn row_to_event(row: PgRow) -> Result<EventEnvelope, EventStoreError> {
    let payload_json: serde_json::Value = row.try_get("payload").map_err(db)?;
    let payload = serde_json::from_value::<EventPayload>(payload_json).map_err(db)?;

    Ok(EventEnvelope {
        meta: EventMeta {
            event_id: row.try_get::<Uuid, _>("event_id").map_err(db)?.to_string(),
            tenant_id: row.try_get::<Uuid, _>("tenant_id").map_err(db)?.to_string(),
            user_id: row.try_get::<Uuid, _>("user_id").map_err(db)?.to_string(),
            agent_id: "default".to_string(),
            session_id: row
                .try_get::<Uuid, _>("session_id")
                .map_err(db)?
                .to_string(),
            sequence_number: i64_to_u64(row.try_get("sequence_number").map_err(db)?)?,
            timestamp_ms: row.try_get::<f64, _>("timestamp_ms").map_err(db)? as i64,
            causation_id: None,
            correlation_id: None,
            producer: EventProducer::TurnExecutor,
            schema_version: 1,
        },
        payload,
    })
}

pub fn row_to_session(row: PgRow) -> Result<Session, EventStoreError> {
    Ok(Session {
        session_id: row.try_get::<Uuid, _>("id").map_err(db)?.to_string(),
        tenant_id: row.try_get::<Uuid, _>("tenant_id").map_err(db)?.to_string(),
        user_id: row.try_get::<Uuid, _>("user_id").map_err(db)?.to_string(),
        agent_id: row.try_get("agent_id").map_err(db)?,
        status: match row.try_get::<String, _>("status").map_err(db)?.as_str() {
            "compacting" => SessionStatus::Compacting,
            "archived" => SessionStatus::Archived,
            _ => SessionStatus::Active,
        },
        title: Some(row.try_get::<String, _>("title").map_err(db)?),
        created_at: row.try_get("created_at").map_err(db)?,
        updated_at: row.try_get("updated_at").map_err(db)?,
        last_active_at: row.try_get("last_active_at").map_err(db)?,
        event_count: i64_to_u64(row.try_get("event_count").map_err(db)?)?,
        last_sequence: i64_to_u64(row.try_get("last_sequence").map_err(db)?)?,
        estimated_prompt_tokens: row
            .try_get::<Option<i32>, _>("estimated_prompt_tokens")
            .map_err(db)?
            .map(|v| v as u32),
        estimated_tokens_after_compaction: None,
        compacted_until_sequence: row
            .try_get::<Option<i64>, _>("compacted_until_sequence")
            .map_err(db)?
            .map(i64_to_u64)
            .transpose()?,
        version: i64_to_u64(row.try_get("version").map_err(db)?)?,
        archived_at: None,
        last_message_at: row
            .try_get::<Option<i64>, _>("last_message_at")
            .map_err(db)?,
        archived: row.try_get("archived").map_err(db)?,
    })
}

pub fn db<E: ToString>(err: E) -> EventStoreError {
    EventStoreError::Database(err.to_string())
}

pub fn map_append_error(err: sqlx::Error) -> EventStoreError {
    if let sqlx::Error::Database(ref db_err) = err
        && db_err.code().as_deref() == Some("23505")
    {
        return EventStoreError::SequenceConflict;
    }
    db(err)
}

pub fn map_create_error(err: sqlx::Error) -> EventStoreError {
    if let sqlx::Error::Database(ref db_err) = err
        && db_err.code().as_deref() == Some("23505")
    {
        return EventStoreError::SessionAlreadyExists("duplicate session".to_string());
    }
    db(err)
}
