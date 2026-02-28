use agent_domain::{
    AppendResult, CreateSessionParams, EventRange, EventStore, EventStoreError, NewEvent,
    SessionListFilter,
};
use agent_domain::{events::EventEnvelope, session::Session};
use sqlx::{PgPool, Postgres, Row, Transaction};
use uuid::Uuid;

use super::event_store_row::{
    db, map_append_error, map_create_error, parse_uuid, row_to_event, row_to_session, u64_to_i64,
};

#[derive(Clone)]
pub struct PostgresEventStore {
    pool: PgPool,
}

impl PostgresEventStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    async fn load_session_lock(
        tx: &mut Transaction<'_, Postgres>,
        session_id: &str,
        session_uuid: Uuid,
    ) -> Result<(Uuid, Uuid, i64, i64), EventStoreError> {
        let lock = sqlx::query(
            "SELECT tenant_id, user_id, last_sequence, event_count
             FROM sessions WHERE id = $1 FOR UPDATE",
        )
        .bind(session_uuid)
        .fetch_optional(&mut **tx)
        .await
        .map_err(db)?
        .ok_or_else(|| EventStoreError::SessionNotFound(session_id.to_string()))?;

        Ok((
            lock.try_get("tenant_id").map_err(db)?,
            lock.try_get("user_id").map_err(db)?,
            lock.try_get("last_sequence").map_err(db)?,
            lock.try_get("event_count").map_err(db)?,
        ))
    }

    async fn insert_events(
        tx: &mut Transaction<'_, Postgres>,
        session_uuid: Uuid,
        tenant_id: Uuid,
        user_id: Uuid,
        start_seq: i64,
        events: &[NewEvent],
    ) -> Result<i64, EventStoreError> {
        let mut next_seq = start_seq;
        for event in events {
            next_seq += 1;
            sqlx::query(
                "INSERT INTO events (event_id, session_id, sequence_number, event_type, payload, tenant_id, user_id)
                 VALUES ($1, $2, $3, $4, $5::jsonb, $6, $7)",
            )
            .bind(parse_uuid(&event.event_id)?)
            .bind(session_uuid)
            .bind(next_seq)
            .bind(&event.event_type)
            .bind(event.payload.to_string())
            .bind(parse_uuid(&event.tenant_id).unwrap_or(tenant_id))
            .bind(parse_uuid(&event.user_id).unwrap_or(user_id))
            .execute(&mut **tx)
            .await
            .map_err(map_append_error)?;
        }

        Ok(next_seq)
    }

    async fn update_session_counters(
        tx: &mut Transaction<'_, Postgres>,
        session_uuid: Uuid,
        next_seq: i64,
        new_count: i64,
    ) -> Result<(), EventStoreError> {
        sqlx::query(
            "UPDATE sessions
             SET last_sequence = $2,
                 event_count = $3,
                 version = version + 1,
                 updated_at = NOW(),
                 last_message_at = NOW()
             WHERE id = $1",
        )
        .bind(session_uuid)
        .bind(next_seq)
        .bind(new_count)
        .execute(&mut **tx)
        .await
        .map_err(db)?;

        Ok(())
    }
}

#[async_trait::async_trait]
impl EventStore for PostgresEventStore {
    async fn append_events(
        &self,
        session_id: &str,
        events: Vec<NewEvent>,
    ) -> Result<AppendResult, EventStoreError> {
        let session_uuid = parse_uuid(session_id)?;
        let mut tx = self.pool.begin().await.map_err(db)?;

        let (tenant_id, user_id, old_last_sequence, old_count) =
            Self::load_session_lock(&mut tx, session_id, session_uuid).await?;

        let next_seq = Self::insert_events(
            &mut tx,
            session_uuid,
            tenant_id,
            user_id,
            old_last_sequence,
            &events,
        )
        .await?;

        let new_count = old_count + i64::try_from(events.len()).map_err(db)?;
        Self::update_session_counters(&mut tx, session_uuid, next_seq, new_count).await?;

        tx.commit().await.map_err(db)?;

        Ok(AppendResult {
            last_sequence: u64::try_from(next_seq).map_err(db)?,
            session_event_count: u64::try_from(new_count).map_err(db)?,
        })
    }

    async fn read_events(
        &self,
        session_id: &str,
        range: EventRange,
    ) -> Result<Vec<EventEnvelope>, EventStoreError> {
        let rows = sqlx::query(
            "SELECT event_id, session_id, sequence_number, event_type, payload, tenant_id, user_id,
                    (EXTRACT(EPOCH FROM created_at) * 1000)::FLOAT8 AS timestamp_ms
             FROM events
             WHERE session_id = $1
               AND ($2::BIGINT IS NULL OR sequence_number >= $2)
               AND ($3::BIGINT IS NULL OR sequence_number <= $3)
             ORDER BY sequence_number ASC",
        )
        .bind(parse_uuid(session_id)?)
        .bind(u64_to_i64(range.start_inclusive)?)
        .bind(u64_to_i64(range.end_inclusive)?)
        .fetch_all(&self.pool)
        .await
        .map_err(db)?;

        rows.into_iter().map(row_to_event).collect()
    }

    async fn read_recent_events(
        &self,
        session_id: &str,
        limit: usize,
    ) -> Result<Vec<EventEnvelope>, EventStoreError> {
        let mut rows = sqlx::query(
            "SELECT event_id, session_id, sequence_number, event_type, payload, tenant_id, user_id,
                    (EXTRACT(EPOCH FROM created_at) * 1000)::FLOAT8 AS timestamp_ms
             FROM events
             WHERE session_id = $1
             ORDER BY sequence_number DESC LIMIT $2",
        )
        .bind(parse_uuid(session_id)?)
        .bind(i64::try_from(limit.min(500)).map_err(db)?)
        .fetch_all(&self.pool)
        .await
        .map_err(db)?;

        rows.reverse();
        rows.into_iter().map(row_to_event).collect()
    }

    async fn get_session(&self, session_id: &str) -> Result<Option<Session>, EventStoreError> {
        let row = sqlx::query(&format!("{} WHERE id = $1", SESSION_SELECT_BASE))
            .bind(parse_uuid(session_id)?)
            .fetch_optional(&self.pool)
            .await
            .map_err(db)?;

        row.map(row_to_session).transpose()
    }

    async fn create_session(
        &self,
        params: CreateSessionParams,
    ) -> Result<Session, EventStoreError> {
        let row = sqlx::query(
            "INSERT INTO sessions (tenant_id, user_id, agent_id, title)
             VALUES ($1, $2, $3, COALESCE($4, '')) RETURNING id, tenant_id, user_id, agent_id,
             status, title, EXTRACT(EPOCH FROM created_at)::BIGINT AS created_at,
             EXTRACT(EPOCH FROM updated_at)::BIGINT AS updated_at,
             EXTRACT(EPOCH FROM COALESCE(last_message_at, created_at))::BIGINT AS last_active_at,
             event_count, last_sequence, estimated_prompt_tokens, compacted_until_sequence, version,
             EXTRACT(EPOCH FROM last_message_at)::BIGINT AS last_message_at, archived",
        )
        .bind(parse_uuid(&params.tenant_id)?)
        .bind(parse_uuid(&params.user_id)?)
        .bind(params.agent_id)
        .bind(params.title)
        .fetch_one(&self.pool)
        .await
        .map_err(map_create_error)?;

        row_to_session(row)
    }

    async fn list_sessions(
        &self,
        filter: SessionListFilter,
    ) -> Result<Vec<Session>, EventStoreError> {
        let rows = sqlx::query(&format!("{} {}", SESSION_SELECT_BASE, SESSION_LIST_SUFFIX))
            .bind(parse_uuid(&filter.user_id)?)
            .bind(parse_uuid(&filter.tenant_id)?)
            .bind(filter.include_archived)
            .bind(i64::from(filter.limit))
            .bind(i64::from(filter.offset))
            .fetch_all(&self.pool)
            .await
            .map_err(db)?;

        rows.into_iter().map(row_to_session).collect()
    }
}

const SESSION_SELECT_BASE: &str = "SELECT id, tenant_id, user_id, agent_id, status, title,
            EXTRACT(EPOCH FROM created_at)::BIGINT AS created_at,
            EXTRACT(EPOCH FROM updated_at)::BIGINT AS updated_at,
            EXTRACT(EPOCH FROM COALESCE(last_message_at, created_at))::BIGINT AS last_active_at,
            event_count, last_sequence, estimated_prompt_tokens, compacted_until_sequence, version,
            EXTRACT(EPOCH FROM last_message_at)::BIGINT AS last_message_at, archived
     FROM sessions";

const SESSION_LIST_SUFFIX: &str =
    "WHERE user_id = $1 AND tenant_id = $2 AND ($3::BOOLEAN = TRUE OR archived = FALSE)
      ORDER BY updated_at DESC LIMIT $4 OFFSET $5";

#[cfg(test)]
#[path = "event_store_tests.rs"]
mod event_store_tests;
