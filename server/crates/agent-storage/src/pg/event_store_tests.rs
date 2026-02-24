use agent_domain::events::{EventPayload, SystemActor, SystemEvent, SystemEventKind};
use agent_domain::{CreateSessionParams, EventRange, EventStore, NewEvent, SessionListFilter};
use sqlx::PgPool;
use uuid::Uuid;

use super::PostgresEventStore;

#[sqlx::test(migrations = "./migrations")]
async fn create_session_and_get(pool: PgPool) {
    let store = PostgresEventStore::new(pool.clone());
    let user_id = create_user(&pool).await;
    let tenant_id = user_id.clone();

    let created = store
        .create_session(CreateSessionParams {
            tenant_id,
            user_id: user_id.clone(),
            agent_id: "agent-a".to_string(),
            title: Some("My Session".to_string()),
        })
        .await
        .expect("create session");

    let loaded = store
        .get_session(&created.session_id)
        .await
        .expect("get session")
        .expect("session exists");

    assert_eq!(loaded.session_id, created.session_id);
    assert_eq!(loaded.user_id, user_id);
}

#[sqlx::test(migrations = "./migrations")]
async fn append_and_read_events(pool: PgPool) {
    let store = PostgresEventStore::new(pool.clone());
    let session_id = seed_session(&pool, &store).await;

    store
        .append_events(
            &session_id,
            vec![new_system_event(), new_system_event(), new_system_event()],
        )
        .await
        .expect("append events");

    let events = store
        .read_events(
            &session_id,
            EventRange {
                start_inclusive: Some(2),
                end_inclusive: Some(3),
            },
        )
        .await
        .expect("read range");

    assert_eq!(events.len(), 2);
    assert_eq!(events[0].meta.sequence_number, 2);
    assert_eq!(events[1].meta.sequence_number, 3);
}

#[sqlx::test(migrations = "./migrations")]
async fn read_recent_events_ordering(pool: PgPool) {
    let store = PostgresEventStore::new(pool.clone());
    let session_id = seed_session(&pool, &store).await;

    let events = (0..5).map(|_| new_system_event()).collect();
    store
        .append_events(&session_id, events)
        .await
        .expect("append events");

    let recent = store
        .read_recent_events(&session_id, 3)
        .await
        .expect("read recent");

    assert_eq!(recent.len(), 3);
    assert_eq!(recent[0].meta.sequence_number, 3);
    assert_eq!(recent[1].meta.sequence_number, 4);
    assert_eq!(recent[2].meta.sequence_number, 5);
}

#[sqlx::test(migrations = "./migrations")]
async fn append_updates_session_counters(pool: PgPool) {
    let store = PostgresEventStore::new(pool.clone());
    let session_id = seed_session(&pool, &store).await;

    let result = store
        .append_events(&session_id, vec![new_system_event(), new_system_event()])
        .await
        .expect("append events");

    let session = store
        .get_session(&session_id)
        .await
        .expect("get")
        .expect("exists");

    assert_eq!(result.last_sequence, 2);
    assert_eq!(result.session_event_count, 2);
    assert_eq!(session.event_count, 2);
    assert_eq!(session.last_sequence, 2);
    assert_eq!(session.version, 1);
}

#[sqlx::test(migrations = "./migrations")]
async fn list_sessions_filters(pool: PgPool) {
    let store = PostgresEventStore::new(pool.clone());
    let user_a = create_user(&pool).await;
    let user_b = create_user(&pool).await;

    let sa = store
        .create_session(CreateSessionParams {
            tenant_id: user_a.clone(),
            user_id: user_a.clone(),
            agent_id: "agent".to_string(),
            title: Some("A".to_string()),
        })
        .await
        .expect("create a");
    let _sb = store
        .create_session(CreateSessionParams {
            tenant_id: user_b.clone(),
            user_id: user_b.clone(),
            agent_id: "agent".to_string(),
            title: Some("B".to_string()),
        })
        .await
        .expect("create b");

    sqlx::query("UPDATE sessions SET archived = TRUE WHERE id = $1")
        .bind(Uuid::parse_str(&sa.session_id).expect("uuid"))
        .execute(&pool)
        .await
        .expect("archive one");

    let only_active = store
        .list_sessions(SessionListFilter {
            user_id: user_a.clone(),
            tenant_id: user_a.clone(),
            include_archived: false,
            limit: 10,
            offset: 0,
        })
        .await
        .expect("list active");
    assert!(only_active.is_empty());

    let with_archived = store
        .list_sessions(SessionListFilter {
            user_id: user_a.clone(),
            tenant_id: user_a,
            include_archived: true,
            limit: 10,
            offset: 0,
        })
        .await
        .expect("list all");
    assert_eq!(with_archived.len(), 1);
    assert_eq!(with_archived[0].user_id, with_archived[0].tenant_id);
}

async fn create_user(pool: &PgPool) -> String {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO users (id, email, password_hash, display_name) VALUES ($1, $2, $3, $4)",
    )
    .bind(id)
    .bind(format!("{}@example.com", id))
    .bind("hash")
    .bind("User")
    .execute(pool)
    .await
    .expect("insert user");
    id.to_string()
}

async fn seed_session(pool: &PgPool, store: &PostgresEventStore) -> String {
    let user_id = create_user(pool).await;
    store
        .create_session(CreateSessionParams {
            tenant_id: user_id.clone(),
            user_id,
            agent_id: "agent".to_string(),
            title: None,
        })
        .await
        .expect("create session")
        .session_id
}

fn new_system_event() -> NewEvent {
    let payload = serde_json::to_value(EventPayload::SystemEvent(SystemEvent {
        kind: SystemEventKind::TurnCompleted,
        actor: SystemActor::Lifecycle,
        detail_json: "{}".to_string(),
    }))
    .expect("payload");

    NewEvent {
        event_id: Uuid::new_v4().to_string(),
        event_type: "SystemEvent".to_string(),
        payload,
        tenant_id: Uuid::new_v4().to_string(),
        user_id: Uuid::new_v4().to_string(),
    }
}
