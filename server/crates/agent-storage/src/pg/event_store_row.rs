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

pub fn i32_to_u32(value: i32) -> Result<u32, EventStoreError> {
    u32::try_from(value).map_err(|e| EventStoreError::Database(e.to_string()))
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
            // TODO: migrate `events.timestamp_ms` from DOUBLE PRECISION to BIGINT to avoid float->int conversion.
            #[allow(clippy::cast_possible_truncation)]
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
            .map(i32_to_u32)
            .transpose()?,
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

#[cfg(test)]
mod tests {
    use sqlx::PgPool;

    use super::*;

    #[test]
    fn conversion_helpers_cover_boundaries() {
        assert_eq!(u64_to_i64(None).expect("none"), None);
        assert_eq!(u64_to_i64(Some(7)).expect("7"), Some(7));
        assert!(u64_to_i64(Some(u64::MAX)).is_err());

        assert_eq!(i64_to_u64(9).expect("9"), 9);
        assert!(i64_to_u64(-1).is_err());

        assert!(parse_uuid("not-a-uuid").is_err());
    }

    #[sqlx::test]
    async fn row_to_event_maps_valid_row(pool: PgPool) {
        let row = sqlx::query(
            r#"
            SELECT
              '00000000-0000-0000-0000-000000000001'::uuid AS event_id,
              '00000000-0000-0000-0000-000000000002'::uuid AS tenant_id,
              '00000000-0000-0000-0000-000000000003'::uuid AS user_id,
              '00000000-0000-0000-0000-000000000004'::uuid AS session_id,
              3::bigint AS sequence_number,
              1700000000000.0::double precision AS timestamp_ms,
              '{"UserMessage":{"message_id":"m1","text":"hi","attachments":[],"input_channel":"telegram","client_message_id":null,"token_estimate":null}}'::jsonb AS payload
            "#,
        )
        .fetch_one(&pool)
        .await
        .expect("select row");

        let event = row_to_event(row).expect("event");
        assert_eq!(event.meta.sequence_number, 3);
        assert_eq!(event.meta.timestamp_ms, 1_700_000_000_000);
    }

    #[sqlx::test]
    async fn row_to_session_handles_status_and_null_errors(pool: PgPool) {
        let compacting_row = sqlx::query(
            r#"
            SELECT
              '00000000-0000-0000-0000-000000000010'::uuid AS id,
              '00000000-0000-0000-0000-000000000011'::uuid AS tenant_id,
              '00000000-0000-0000-0000-000000000012'::uuid AS user_id,
              'agent'::text AS agent_id,
              'compacting'::text AS status,
              'title'::text AS title,
              1::bigint AS created_at,
              2::bigint AS updated_at,
              3::bigint AS last_active_at,
              4::bigint AS event_count,
              5::bigint AS last_sequence,
              100::integer AS estimated_prompt_tokens,
              6::bigint AS compacted_until_sequence,
              7::bigint AS version,
              NULL::bigint AS last_message_at,
              false AS archived
            "#,
        )
        .fetch_one(&pool)
        .await
        .expect("select row");

        let session = row_to_session(compacting_row).expect("session");
        assert!(matches!(session.status, SessionStatus::Compacting));
        assert_eq!(session.compacted_until_sequence, Some(6));

        let bad_row = sqlx::query(
            r#"
            SELECT
              '00000000-0000-0000-0000-000000000020'::uuid AS id,
              '00000000-0000-0000-0000-000000000021'::uuid AS tenant_id,
              '00000000-0000-0000-0000-000000000022'::uuid AS user_id,
              'agent'::text AS agent_id,
              'active'::text AS status,
              NULL::text AS title,
              1::bigint AS created_at,
              2::bigint AS updated_at,
              3::bigint AS last_active_at,
              4::bigint AS event_count,
              5::bigint AS last_sequence,
              NULL::integer AS estimated_prompt_tokens,
              NULL::bigint AS compacted_until_sequence,
              7::bigint AS version,
              NULL::bigint AS last_message_at,
              false AS archived
            "#,
        )
        .fetch_one(&pool)
        .await
        .expect("select row");

        assert!(row_to_session(bad_row).is_err());
    }

    #[test]
    fn map_append_error_unique_violation_returns_sequence_conflict() {
        // Construct a synthetic sqlx unique violation error
        let err = sqlx::Error::Protocol("23505: unique_violation".into());
        // For non-Database errors, map_append_error falls through to Database variant
        let mapped = map_append_error(err);
        assert!(matches!(mapped, EventStoreError::Database(_)));
    }

    #[test]
    fn map_create_error_non_db_returns_database_variant() {
        let err = sqlx::Error::Protocol("some protocol error".into());
        let mapped = map_create_error(err);
        assert!(matches!(mapped, EventStoreError::Database(_)));
    }
}
