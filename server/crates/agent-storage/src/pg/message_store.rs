use agent_domain::{MessageStore, StoreError, StoredMessage, StoredSession};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Clone)]
pub struct PostgresMessageStore {
    pool: PgPool,
}

#[derive(sqlx::FromRow)]
struct SessionRow {
    id: Uuid,
}

#[derive(sqlx::FromRow)]
struct SessionDetailRow {
    id: Uuid,
    user_id: Uuid,
    agent_id: String,
    title: String,
    summary: String,
    created_at: i64,
    updated_at: i64,
    last_message_at: Option<i64>,
    archived: bool,
}

#[derive(sqlx::FromRow)]
struct MessageRow {
    id: Uuid,
    session_id: Uuid,
    role: String,
    content: String,
    created_at: i64,
}

impl PostgresMessageStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl MessageStore for PostgresMessageStore {
    async fn create_session(&self, user_id: &str, agent_id: &str) -> Result<String, StoreError> {
        let user_uuid = parse_uuid(user_id, "user_id")?;
        let session = sqlx::query_as::<_, SessionRow>(
            "INSERT INTO sessions (user_id, agent_id) VALUES ($1, $2) RETURNING id",
        )
        .bind(user_uuid)
        .bind(agent_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_sqlx_error)?;

        Ok(session.id.to_string())
    }

    async fn create_session_with_title(
        &self,
        user_id: &str,
        title: &str,
    ) -> Result<StoredSession, StoreError> {
        let user_uuid = parse_uuid(user_id, "user_id")?;
        let row = sqlx::query_as::<_, SessionDetailRow>(
            "INSERT INTO sessions (user_id, agent_id, title)
             VALUES ($1, '', $2)
             RETURNING id, user_id, agent_id, title, summary,
                EXTRACT(EPOCH FROM created_at)::BIGINT AS created_at,
                EXTRACT(EPOCH FROM updated_at)::BIGINT AS updated_at,
                EXTRACT(EPOCH FROM last_message_at)::BIGINT AS last_message_at,
                archived",
        )
        .bind(user_uuid)
        .bind(title)
        .fetch_one(&self.pool)
        .await
        .map_err(map_sqlx_error)?;

        Ok(to_stored_session(row))
    }

    async fn list_sessions(&self, user_id: &str) -> Result<Vec<StoredSession>, StoreError> {
        let user_uuid = parse_uuid(user_id, "user_id")?;
        let rows = sqlx::query_as::<_, SessionDetailRow>(
            "SELECT id, user_id, agent_id, title, summary,
                EXTRACT(EPOCH FROM created_at)::BIGINT AS created_at,
                EXTRACT(EPOCH FROM updated_at)::BIGINT AS updated_at,
                EXTRACT(EPOCH FROM last_message_at)::BIGINT AS last_message_at,
                archived
             FROM sessions
             WHERE user_id = $1 AND archived = false
             ORDER BY updated_at DESC
             LIMIT 50",
        )
        .bind(user_uuid)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_error)?;

        Ok(rows.into_iter().map(to_stored_session).collect())
    }

    async fn get_session(
        &self,
        user_id: &str,
        session_id: &str,
    ) -> Result<Option<StoredSession>, StoreError> {
        let user_uuid = parse_uuid(user_id, "user_id")?;
        let session_uuid = parse_uuid(session_id, "session_id")?;
        let row = sqlx::query_as::<_, SessionDetailRow>(
            "SELECT id, user_id, agent_id, title, summary,
                EXTRACT(EPOCH FROM created_at)::BIGINT AS created_at,
                EXTRACT(EPOCH FROM updated_at)::BIGINT AS updated_at,
                EXTRACT(EPOCH FROM last_message_at)::BIGINT AS last_message_at,
                archived
             FROM sessions
             WHERE id = $1 AND user_id = $2",
        )
        .bind(session_uuid)
        .bind(user_uuid)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_error)?;

        Ok(row.map(to_stored_session))
    }

    async fn save_message(
        &self,
        session_id: &str,
        role: &str,
        content: &str,
    ) -> Result<String, StoreError> {
        let session_uuid = parse_uuid(session_id, "session_id")?;

        let message = sqlx::query_as::<_, SessionRow>(
            "INSERT INTO messages (session_id, role, content)
             VALUES ($1, $2, $3)
             RETURNING id",
        )
        .bind(session_uuid)
        .bind(role)
        .bind(content)
        .fetch_one(&self.pool)
        .await
        .map_err(map_sqlx_error)?;

        sqlx::query(
            "UPDATE sessions
             SET updated_at = NOW(), last_message_at = NOW()
             WHERE id = $1",
        )
        .bind(session_uuid)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_error)?;

        Ok(message.id.to_string())
    }

    async fn get_session_messages(
        &self,
        session_id: &str,
        limit: i64,
    ) -> Result<Vec<StoredMessage>, StoreError> {
        let session_uuid = parse_uuid(session_id, "session_id")?;
        let bounded_limit = limit.clamp(1, 200);

        let mut rows = sqlx::query_as::<_, MessageRow>(
            "SELECT id, session_id, role, content,
                    EXTRACT(EPOCH FROM created_at)::BIGINT AS created_at
             FROM messages
             WHERE session_id = $1
             ORDER BY sequence_num DESC
             LIMIT $2",
        )
        .bind(session_uuid)
        .bind(bounded_limit)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_error)?;

        rows.reverse();

        Ok(rows
            .into_iter()
            .map(|row| StoredMessage {
                id: row.id.to_string(),
                session_id: row.session_id.to_string(),
                role: row.role,
                content: row.content,
                created_at: row.created_at,
            })
            .collect())
    }

    async fn get_or_create_default_session(&self, user_id: &str) -> Result<String, StoreError> {
        let user_uuid = parse_uuid(user_id, "user_id")?;

        let latest = sqlx::query_as::<_, SessionRow>(
            "SELECT id
             FROM sessions
             WHERE user_id = $1 AND archived = FALSE
             ORDER BY COALESCE(last_message_at, created_at) DESC
             LIMIT 1",
        )
        .bind(user_uuid)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_error)?;

        match latest {
            Some(row) => Ok(row.id.to_string()),
            None => self.create_session(user_id, "").await,
        }
    }
}

fn to_stored_session(row: SessionDetailRow) -> StoredSession {
    StoredSession {
        id: row.id.to_string(),
        user_id: row.user_id.to_string(),
        agent_id: row.agent_id,
        title: row.title,
        summary: row.summary,
        created_at: row.created_at,
        updated_at: row.updated_at,
        last_message_at: row.last_message_at,
        archived: row.archived,
    }
}

fn parse_uuid(raw: &str, field: &str) -> Result<Uuid, StoreError> {
    Uuid::parse_str(raw).map_err(|_| StoreError::NotFound(format!("invalid {field}")))
}

fn map_sqlx_error(err: sqlx::Error) -> StoreError {
    StoreError::Internal(err.to_string())
}

#[cfg(test)]
#[path = "message_store_tests.rs"]
mod tests;
