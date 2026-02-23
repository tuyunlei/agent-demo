use agent_domain::AuthPort;
use agent_domain::MessageStore;
use agent_storage::pg::PostgresMessageStore;
use agent_storage::pg::PostgresUserStore;
use sqlx::PgPool;

#[sqlx::test(migrations = "./migrations")]
async fn user_create_and_authenticate(pool: PgPool) {
    let store = PostgresUserStore::new(pool);
    let result = store
        .create_user("alice@test.com", "password123", "Alice")
        .await
        .unwrap();
    assert_eq!(result.display_name, "Alice");
    assert!(!result.user_id.is_empty());

    let auth = store
        .authenticate("alice@test.com", "password123")
        .await
        .unwrap();
    assert_eq!(auth.user_id, result.user_id);
}

#[sqlx::test(migrations = "./migrations")]
async fn user_duplicate_email_fails(pool: PgPool) {
    let store = PostgresUserStore::new(pool);
    store
        .create_user("bob@test.com", "pass1", "Bob")
        .await
        .unwrap();

    let err = store
        .create_user("bob@test.com", "pass2", "Bob2")
        .await
        .unwrap_err();
    assert!(matches!(err, agent_domain::AuthError::AlreadyExists(_)));
}

#[sqlx::test(migrations = "./migrations")]
async fn user_wrong_password_fails(pool: PgPool) {
    let store = PostgresUserStore::new(pool);
    store
        .create_user("carol@test.com", "correct", "Carol")
        .await
        .unwrap();

    let err = store
        .authenticate("carol@test.com", "wrong")
        .await
        .unwrap_err();
    assert!(matches!(err, agent_domain::AuthError::InvalidCredentials));
}

#[sqlx::test(migrations = "./migrations")]
async fn message_store_create_session_and_save(pool: PgPool) {
    let user_store = PostgresUserStore::new(pool.clone());
    let user = user_store
        .create_user("dave@test.com", "pass", "Dave")
        .await
        .unwrap();

    let msg_store = PostgresMessageStore::new(pool);
    let session_id = msg_store
        .create_session(&user.user_id, "agent-1")
        .await
        .unwrap();
    assert!(!session_id.is_empty());

    msg_store
        .save_message(&session_id, "user", "hello")
        .await
        .unwrap();
    msg_store
        .save_message(&session_id, "assistant", "hi there")
        .await
        .unwrap();

    let messages = msg_store
        .get_session_messages(&session_id, 50)
        .await
        .unwrap();
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0].role, "user");
    assert_eq!(messages[0].content, "hello");
    assert_eq!(messages[1].role, "assistant");
    assert_eq!(messages[1].content, "hi there");
}

#[sqlx::test(migrations = "./migrations")]
async fn message_store_create_with_title_and_get(pool: PgPool) {
    let user_store = PostgresUserStore::new(pool.clone());
    let user = user_store
        .create_user("title@test.com", "pass", "Title")
        .await
        .unwrap();

    let msg_store = PostgresMessageStore::new(pool);
    let created = msg_store
        .create_session_with_title(&user.user_id, "Roadmap")
        .await
        .unwrap();
    assert_eq!(created.title, "Roadmap");

    let fetched = msg_store
        .get_session(&user.user_id, &created.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(fetched.id, created.id);
    assert_eq!(fetched.user_id, user.user_id);
}

#[sqlx::test(migrations = "./migrations")]
async fn message_store_list_sessions(pool: PgPool) {
    let user_store = PostgresUserStore::new(pool.clone());
    let user = user_store
        .create_user("list@test.com", "pass", "List")
        .await
        .unwrap();

    let msg_store = PostgresMessageStore::new(pool);
    msg_store
        .create_session_with_title(&user.user_id, "Session A")
        .await
        .unwrap();
    msg_store
        .create_session_with_title(&user.user_id, "Session B")
        .await
        .unwrap();

    let sessions = msg_store.list_sessions(&user.user_id).await.unwrap();
    assert_eq!(sessions.len(), 2);
}

#[sqlx::test(migrations = "./migrations")]
async fn message_store_get_or_create_default_session(pool: PgPool) {
    let user_store = PostgresUserStore::new(pool.clone());
    let user = user_store
        .create_user("eve@test.com", "pass", "Eve")
        .await
        .unwrap();

    let msg_store = PostgresMessageStore::new(pool);
    let s1 = msg_store
        .get_or_create_default_session(&user.user_id)
        .await
        .unwrap();
    let s2 = msg_store
        .get_or_create_default_session(&user.user_id)
        .await
        .unwrap();
    assert_eq!(s1, s2);
}

#[sqlx::test(migrations = "./migrations")]
async fn list_messages_respects_limit(pool: PgPool) {
    let user_store = PostgresUserStore::new(pool.clone());
    let user = user_store
        .create_user("limit@test.com", "pass", "Limit")
        .await
        .unwrap();

    let msg_store = PostgresMessageStore::new(pool);
    let session_id = msg_store
        .create_session(&user.user_id, "agent-1")
        .await
        .unwrap();

    for idx in 1..=10 {
        msg_store
            .save_message(&session_id, "user", &format!("message-{idx}"))
            .await
            .unwrap();
    }

    let messages = msg_store
        .get_session_messages(&session_id, 3)
        .await
        .unwrap();
    assert_eq!(messages.len(), 3);
    assert_eq!(messages[0].content, "message-8");
    assert_eq!(messages[1].content, "message-9");
    assert_eq!(messages[2].content, "message-10");
}

#[sqlx::test(migrations = "./migrations")]
async fn list_messages_returns_in_sequence_order(pool: PgPool) {
    let user_store = PostgresUserStore::new(pool.clone());
    let user = user_store
        .create_user("sequence@test.com", "pass", "Sequence")
        .await
        .unwrap();

    let msg_store = PostgresMessageStore::new(pool);
    let session_id = msg_store
        .create_session(&user.user_id, "agent-1")
        .await
        .unwrap();

    for content in ["first", "second", "third", "fourth"] {
        msg_store
            .save_message(&session_id, "assistant", content)
            .await
            .unwrap();
    }

    let messages = msg_store
        .get_session_messages(&session_id, 50)
        .await
        .unwrap();

    let contents: Vec<&str> = messages.iter().map(|m| m.content.as_str()).collect();
    assert_eq!(contents, vec!["first", "second", "third", "fourth"]);
}

#[sqlx::test(migrations = "./migrations")]
async fn create_user_duplicate_email_returns_error(pool: PgPool) {
    let store = PostgresUserStore::new(pool);

    store
        .create_user("dup-user@test.com", "pass1", "Dupe1")
        .await
        .unwrap();

    let err = store
        .create_user("dup-user@test.com", "pass2", "Dupe2")
        .await
        .unwrap_err();

    assert!(matches!(err, agent_domain::AuthError::AlreadyExists(_)));
}
