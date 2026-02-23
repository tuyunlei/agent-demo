use std::sync::Arc;

use agent_llm::MockLlmProvider;
use agent_server::ServerBuilder;
use agent_storage::pg::{PostgresMessageStore, PostgresUserStore};
use sqlx::PgPool;

use agent_proto::auth_service_client::AuthServiceClient;
use agent_proto::chat_service_client::ChatServiceClient;
use agent_proto::session_service_client::SessionServiceClient;

pub struct TestEnv {
    pub addr: String,
    pub mock_llm: Arc<MockLlmProvider>,
    shutdown_tx: Option<tokio::sync::oneshot::Sender<()>>,
}

impl TestEnv {
    pub async fn start(pool: PgPool) -> Self {
        let mock_llm = Arc::new(MockLlmProvider::with_text("Hello from mock!"));
        let user_store = Arc::new(PostgresUserStore::new(pool.clone()));
        let message_store = Arc::new(PostgresMessageStore::new(pool));

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind random local port");
        let addr = listener.local_addr().expect("read local addr");
        let port = addr.port();
        drop(listener);

        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();

        let server = ServerBuilder::new(
            user_store,
            mock_llm.clone(),
            message_store,
            "e2e-test-jwt-secret",
            addr,
        );

        tokio::spawn(async move {
            server
                .serve_with_shutdown(async {
                    let _ = shutdown_rx.await;
                })
                .await
                .expect("serve_with_shutdown should run");
        });

        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        Self {
            addr: format!("http://127.0.0.1:{port}"),
            mock_llm,
            shutdown_tx: Some(shutdown_tx),
        }
    }

    pub async fn auth_client(&self) -> AuthServiceClient<tonic::transport::Channel> {
        AuthServiceClient::connect(self.addr.clone())
            .await
            .expect("connect auth client")
    }

    pub async fn chat_client(&self) -> ChatServiceClient<tonic::transport::Channel> {
        ChatServiceClient::connect(self.addr.clone())
            .await
            .expect("connect chat client")
    }

    pub async fn session_client(&self) -> SessionServiceClient<tonic::transport::Channel> {
        SessionServiceClient::connect(self.addr.clone())
            .await
            .expect("connect session client")
    }
}

impl Drop for TestEnv {
    fn drop(&mut self) {
        if let Some(shutdown_tx) = self.shutdown_tx.take() {
            let _ = shutdown_tx.send(());
        }
    }
}
