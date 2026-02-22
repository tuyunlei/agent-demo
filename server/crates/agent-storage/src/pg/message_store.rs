use agent_domain::{MessageStore, StoreError, StoredMessage};
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
             ORDER BY created_at DESC
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

fn parse_uuid(raw: &str, field: &str) -> Result<Uuid, StoreError> {
    Uuid::parse_str(raw).map_err(|_| StoreError::NotFound(format!("invalid {field}")))
}

fn map_sqlx_error(err: sqlx::Error) -> StoreError {
    StoreError::Internal(err.to_string())
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use agent_domain::MessageStore;

    use super::*;

    #[derive(Default)]
    struct InMemoryMessageMock {
        sessions: Arc<Mutex<Vec<String>>>,
        messages: Arc<Mutex<Vec<StoredMessage>>>,
    }

    #[async_trait::async_trait]
    impl MessageStore for InMemoryMessageMock {
        async fn create_session(
            &self,
            user_id: &str,
            _agent_id: &str,
        ) -> Result<String, StoreError> {
            let id = format!("session-{user_id}");
            self.sessions
                .lock()
                .expect("lock sessions")
                .push(id.clone());
            Ok(id)
        }

        async fn save_message(
            &self,
            session_id: &str,
            role: &str,
            content: &str,
        ) -> Result<String, StoreError> {
            let id = format!("msg-{}", self.messages.lock().expect("lock").len() + 1);
            self.messages
                .lock()
                .expect("lock messages")
                .push(StoredMessage {
                    id: id.clone(),
                    session_id: session_id.to_string(),
                    role: role.to_string(),
                    content: content.to_string(),
                    created_at: 1,
                });
            Ok(id)
        }

        async fn get_session_messages(
            &self,
            session_id: &str,
            _limit: i64,
        ) -> Result<Vec<StoredMessage>, StoreError> {
            let items = self
                .messages
                .lock()
                .expect("lock messages")
                .iter()
                .filter(|item| item.session_id == session_id)
                .cloned()
                .collect();
            Ok(items)
        }

        async fn get_or_create_default_session(&self, user_id: &str) -> Result<String, StoreError> {
            let first = self
                .sessions
                .lock()
                .expect("lock sessions")
                .first()
                .cloned();
            match first {
                Some(id) => Ok(id),
                None => self.create_session(user_id, "").await,
            }
        }
    }

    #[tokio::test]
    async fn save_message_mock_works() {
        let store = InMemoryMessageMock::default();
        let session_id = store.create_session("user-1", "").await.expect("session");

        let id = store
            .save_message(&session_id, "user", "hello")
            .await
            .expect("saved");

        assert_eq!(id, "msg-1");
    }

    #[tokio::test]
    async fn get_messages_mock_works() {
        let store = InMemoryMessageMock::default();
        let session_id = store.create_session("user-2", "").await.expect("session");
        store
            .save_message(&session_id, "user", "hello")
            .await
            .expect("saved");

        let messages = store
            .get_session_messages(&session_id, 50)
            .await
            .expect("messages");

        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].content, "hello");
    }

    #[tokio::test]
    async fn default_session_creation_mock_works() {
        let store = InMemoryMessageMock::default();

        let session_id = store
            .get_or_create_default_session("user-3")
            .await
            .expect("session");

        assert_eq!(session_id, "session-user-3");
    }
}
