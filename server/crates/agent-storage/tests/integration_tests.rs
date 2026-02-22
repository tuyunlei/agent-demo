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
