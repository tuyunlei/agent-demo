mod common;

use agent_llm::error::LlmError;
use agent_proto::content_block::Kind;
use agent_proto::{
    ContentBlock, ListSessionMessagesRequest, LoginRequest, RegisterRequest, SendMessageRequest,
    TextBlock,
};
use common::TestEnv;
use sqlx::PgPool;

fn text_block(text: &str) -> ContentBlock {
    ContentBlock {
        kind: Some(Kind::Text(TextBlock {
            text: text.to_string(),
        })),
    }
}

fn bearer(token: &str) -> tonic::metadata::MetadataValue<tonic::metadata::Ascii> {
    format!("Bearer {token}")
        .parse()
        .expect("valid bearer header")
}

async fn register_and_get_access_token(env: &TestEnv, email: &str, display_name: &str) -> String {
    let mut auth = env.auth_client().await;
    auth.register(RegisterRequest {
        invite_code: String::new(),
        email: email.to_string(),
        password: "pass123".into(),
        display_name: display_name.to_string(),
    })
    .await
    .expect("register should succeed")
    .into_inner()
    .token_pair
    .expect("token pair should exist")
    .access_token
}

#[sqlx::test(migrations = "../agent-storage/migrations")]
async fn register_returns_user_id_and_tokens(pool: PgPool) {
    let env = TestEnv::start(pool).await;
    let mut client = env.auth_client().await;

    let resp = client
        .register(RegisterRequest {
            invite_code: String::new(),
            email: "test@e2e.local".into(),
            password: "password123".into(),
            display_name: "E2E User".into(),
        })
        .await
        .expect("register should succeed")
        .into_inner();

    assert!(!resp.user_id.is_empty());
    assert!(resp.token_pair.is_some());
    let tokens = resp.token_pair.expect("token pair should exist");
    assert!(!tokens.access_token.is_empty());
    assert!(!tokens.refresh_token.is_empty());
}

#[sqlx::test(migrations = "../agent-storage/migrations")]
async fn login_after_register(pool: PgPool) {
    let env = TestEnv::start(pool).await;
    let mut client = env.auth_client().await;

    client
        .register(RegisterRequest {
            invite_code: String::new(),
            email: "login@e2e.local".into(),
            password: "pass123".into(),
            display_name: "Login User".into(),
        })
        .await
        .expect("register should succeed");

    let resp = client
        .login(LoginRequest {
            email: "login@e2e.local".into(),
            password: "pass123".into(),
            device_name: "e2e-test".into(),
            platform: "linux".into(),
        })
        .await
        .expect("login should succeed")
        .into_inner();

    assert!(!resp.user_id.is_empty());
    assert!(resp.token_pair.is_some());
}

#[sqlx::test(migrations = "../agent-storage/migrations")]
async fn duplicate_register_returns_already_exists(pool: PgPool) {
    let env = TestEnv::start(pool).await;
    let mut client = env.auth_client().await;

    let req = RegisterRequest {
        invite_code: String::new(),
        email: "dup@e2e.local".into(),
        password: "pass123".into(),
        display_name: "Dup".into(),
    };

    client
        .register(req.clone())
        .await
        .expect("first register should succeed");
    let err = client
        .register(req)
        .await
        .expect_err("duplicate should fail");
    assert_eq!(err.code(), tonic::Code::AlreadyExists);
}

#[sqlx::test(migrations = "../agent-storage/migrations")]
async fn invalid_token_rejected(pool: PgPool) {
    let env = TestEnv::start(pool).await;
    let mut client = env.chat_client().await;

    let mut req = tonic::Request::new(SendMessageRequest {
        request_id: "e2e-invalid".into(),
        session_id: String::new(),
        agent_id: "default".into(),
        content: vec![text_block("Hi")],
        metadata: Default::default(),
    });
    req.metadata_mut().insert(
        "authorization",
        "Bearer fake-token".parse().expect("valid metadata"),
    );

    let err = client
        .send_message(req)
        .await
        .expect_err("invalid token should fail");
    assert_eq!(err.code(), tonic::Code::Unauthenticated);
}

#[sqlx::test(migrations = "../agent-storage/migrations")]
async fn full_chat_roundtrip(pool: PgPool) {
    let env = TestEnv::start(pool).await;

    let token = register_and_get_access_token(&env, "chat@e2e.local", "Chat User").await;

    let mut chat = env.chat_client().await;
    let mut req = tonic::Request::new(SendMessageRequest {
        request_id: "e2e-chat-1".into(),
        session_id: String::new(),
        agent_id: "default".into(),
        content: vec![text_block("Hi there")],
        metadata: Default::default(),
    });
    req.metadata_mut().insert("authorization", bearer(&token));

    let resp = chat
        .send_message(req)
        .await
        .expect("chat should succeed")
        .into_inner();
    assert!(!resp.session_id.is_empty());
    assert!(!resp.assistant_content.is_empty());
    let first_reply_text = match resp.assistant_content.first().and_then(|b| b.kind.as_ref()) {
        Some(Kind::Text(text)) => text.text.clone(),
        _ => String::new(),
    };
    assert_eq!(first_reply_text, "Hello from mock!");

    let mut session = env.session_client().await;
    let mut req = tonic::Request::new(ListSessionMessagesRequest {
        session_id: resp.session_id.clone(),
        pagination: None,
        created_at_order: 0,
    });
    req.metadata_mut().insert("authorization", bearer(&token));

    let history = session
        .list_session_messages(req)
        .await
        .expect("list messages should succeed")
        .into_inner();
    assert!(history.messages.len() >= 2);

    assert_eq!(env.mock_llm.calls().len(), 1);
}

#[sqlx::test(migrations = "../agent-storage/migrations")]
async fn llm_error_returns_internal(pool: PgPool) {
    let env = TestEnv::start(pool).await;
    env.mock_llm
        .set_response(Err(LlmError::Internal("mock provider error".into())));

    let token = register_and_get_access_token(&env, "err@e2e.local", "Err User").await;

    let mut chat = env.chat_client().await;
    let mut req = tonic::Request::new(SendMessageRequest {
        request_id: "e2e-err-1".into(),
        session_id: String::new(),
        agent_id: "default".into(),
        content: vec![text_block("trigger error")],
        metadata: Default::default(),
    });
    req.metadata_mut().insert("authorization", bearer(&token));

    let err = chat
        .send_message(req)
        .await
        .expect_err("llm error should fail");
    assert_eq!(err.code(), tonic::Code::Internal);
}
