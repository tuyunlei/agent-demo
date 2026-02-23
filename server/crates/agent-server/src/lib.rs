use std::future::Future;
use std::net::SocketAddr;
use std::sync::Arc;

use agent_app::{AgentRuntime, AuthService, BuiltinToolRuntime};
use agent_channel::{
    AuthServiceHandler, ChatServiceHandler, SessionServiceHandler, auth_interceptor,
};
use agent_domain::{AuthPort, LlmProvider, MessageStore};
use agent_proto::auth_service_server::AuthServiceServer;
use agent_proto::chat_service_server::ChatServiceServer;
use agent_proto::session_service_server::SessionServiceServer;
use tonic::transport::Server;

pub struct ServerBuilder {
    auth_port: Arc<dyn AuthPort>,
    llm_provider: Arc<dyn LlmProvider>,
    message_store: Arc<dyn MessageStore>,
    jwt_secret: String,
    addr: SocketAddr,
}

impl ServerBuilder {
    pub fn new(
        auth_port: Arc<dyn AuthPort>,
        llm_provider: Arc<dyn LlmProvider>,
        message_store: Arc<dyn MessageStore>,
        jwt_secret: impl Into<String>,
        addr: SocketAddr,
    ) -> Self {
        Self {
            auth_port,
            llm_provider,
            message_store,
            jwt_secret: jwt_secret.into(),
            addr,
        }
    }

    pub async fn serve(self) -> Result<(), tonic::transport::Error> {
        self.serve_with_shutdown(std::future::pending()).await
    }

    pub async fn serve_with_shutdown(
        self,
        signal: impl Future<Output = ()>,
    ) -> Result<(), tonic::transport::Error> {
        let auth_service = Arc::new(AuthService::new(self.auth_port, self.jwt_secret));
        let runtime = Arc::new(AgentRuntime::new(
            self.llm_provider,
            self.message_store.clone(),
            Arc::new(BuiltinToolRuntime::new()),
        ));

        let chat_service = ChatServiceServer::with_interceptor(
            ChatServiceHandler::new(runtime),
            auth_interceptor(auth_service.clone()),
        );
        let session_service = SessionServiceServer::with_interceptor(
            SessionServiceHandler::new(self.message_store),
            auth_interceptor(auth_service.clone()),
        );
        let auth_service = AuthServiceServer::new(AuthServiceHandler::new(auth_service));

        Server::builder()
            .add_service(chat_service)
            .add_service(session_service)
            .add_service(auth_service)
            .serve_with_shutdown(self.addr, signal)
            .await
    }
}
