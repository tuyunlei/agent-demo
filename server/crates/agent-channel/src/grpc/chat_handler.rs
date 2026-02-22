use std::pin::Pin;
use std::sync::Arc;

use super::UserId;
use agent_app::AgentRuntime;
use agent_domain::{AgentError, LlmError, StoreError};
use agent_proto::chat_service_server::ChatService;
use agent_proto::{
    ChatEvent, SendMessageRequest, SendMessageResponse, SubmitToolResultRequest,
    SubmitToolResultResponse, SubscribeRequest, content_block,
};
use tonic::{Request, Response, Status};

pub struct ChatServiceHandler {
    runtime: Arc<AgentRuntime>,
}

impl ChatServiceHandler {
    pub fn new(runtime: Arc<AgentRuntime>) -> Self {
        Self { runtime }
    }
}

#[tonic::async_trait]
impl ChatService for ChatServiceHandler {
    async fn send_message(
        &self,
        request: Request<SendMessageRequest>,
    ) -> Result<Response<SendMessageResponse>, Status> {
        let user_id = request
            .extensions()
            .get::<UserId>()
            .map(|id| id.0.clone())
            .unwrap_or_else(|| "unknown".to_string());
        let req = request.into_inner();
        println!(
            "Received SendMessage: request_id={}, user_id={}",
            req.request_id, user_id
        );

        let user_text = extract_text(&req)?;
        let result = self
            .runtime
            .handle_message(&user_id, non_empty(&req.session_id), &user_text)
            .await
            .map_err(map_agent_error)?;

        use agent_proto::ContentBlock;
        use agent_proto::TextBlock;
        use agent_proto::content_block::Kind;

        Ok(Response::new(SendMessageResponse {
            request_id: req.request_id,
            session_id: result.session_id,
            user_message_id: "msg-001".to_string(),
            assistant_content: vec![ContentBlock {
                kind: Some(Kind::Text(TextBlock { text: result.reply })),
            }],
        }))
    }

    type SubscribeStream =
        Pin<Box<dyn tokio_stream::Stream<Item = Result<ChatEvent, Status>> + Send>>;

    async fn subscribe(
        &self,
        _request: Request<SubscribeRequest>,
    ) -> Result<Response<Self::SubscribeStream>, Status> {
        Err(Status::unimplemented("not implemented"))
    }

    async fn submit_tool_result(
        &self,
        _request: Request<SubmitToolResultRequest>,
    ) -> Result<Response<SubmitToolResultResponse>, Status> {
        Err(Status::unimplemented("not implemented"))
    }
}

#[allow(clippy::result_large_err)]
fn extract_text(req: &SendMessageRequest) -> Result<String, Status> {
    let text_parts = req
        .content
        .iter()
        .filter_map(|block| match &block.kind {
            Some(content_block::Kind::Text(text_block)) => Some(text_block.text.clone()),
            _ => None,
        })
        .collect::<Vec<_>>();

    let text = text_parts.join("\n");
    if text.trim().is_empty() {
        return Err(Status::invalid_argument(
            "SendMessage.content must include at least one non-empty text block",
        ));
    }

    Ok(text)
}

fn map_agent_error(err: AgentError) -> Status {
    match err {
        AgentError::InvalidInput(msg) => Status::invalid_argument(msg),
        AgentError::Llm(llm_err) => map_llm_error(llm_err),
        AgentError::Store(store_err) => map_store_error(store_err),
    }
}

fn non_empty(value: &str) -> Option<&str> {
    if value.trim().is_empty() {
        None
    } else {
        Some(value)
    }
}

fn map_llm_error(err: LlmError) -> Status {
    match err {
        LlmError::RateLimited => Status::resource_exhausted("llm request rate limited"),
        LlmError::InvalidRequest(msg) => Status::invalid_argument(msg),
        LlmError::Timeout => Status::deadline_exceeded("llm request timeout"),
        LlmError::ProviderError(msg) => Status::internal(msg),
    }
}

fn map_store_error(err: StoreError) -> Status {
    match err {
        StoreError::NotFound(msg) => Status::not_found(msg),
        StoreError::Internal(msg) => Status::internal(msg),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use agent_domain::{
        LlmProvider, LlmRequest, LlmResponse, LlmUsage, MessageStore, StoreError, StoredMessage,
    };
    use agent_proto::{ContentBlock, TextBlock};

    use super::*;

    struct MockLlmProvider {
        captured: Arc<Mutex<Vec<LlmRequest>>>,
    }

    #[async_trait::async_trait]
    impl LlmProvider for MockLlmProvider {
        async fn generate(&self, request: LlmRequest) -> Result<LlmResponse, LlmError> {
            self.captured.lock().expect("lock captured").push(request);
            Ok(LlmResponse {
                content: "mocked-reply".to_string(),
                model: "mock-model".to_string(),
                usage: Some(LlmUsage {
                    input_tokens: 10,
                    output_tokens: 10,
                    total_tokens: 20,
                }),
            })
        }
    }

    #[derive(Default)]
    struct MockMessageStore;

    #[async_trait::async_trait]
    impl MessageStore for MockMessageStore {
        async fn create_session(
            &self,
            user_id: &str,
            _agent_id: &str,
        ) -> Result<String, StoreError> {
            Ok(format!("session-{user_id}"))
        }

        async fn save_message(
            &self,
            _session_id: &str,
            _role: &str,
            _content: &str,
        ) -> Result<String, StoreError> {
            Ok("msg-id".to_string())
        }

        async fn get_session_messages(
            &self,
            session_id: &str,
            _limit: i64,
        ) -> Result<Vec<StoredMessage>, StoreError> {
            Ok(vec![StoredMessage {
                id: "m1".to_string(),
                session_id: session_id.to_string(),
                role: "user".to_string(),
                content: "hello runtime".to_string(),
                created_at: 1,
            }])
        }

        async fn get_or_create_default_session(&self, user_id: &str) -> Result<String, StoreError> {
            Ok(format!("session-{user_id}"))
        }
    }

    fn text_block(text: &str) -> ContentBlock {
        ContentBlock {
            kind: Some(content_block::Kind::Text(TextBlock {
                text: text.to_string(),
            })),
        }
    }

    #[tokio::test]
    async fn test_send_message_calls_runtime_chain() {
        let captured = Arc::new(Mutex::new(Vec::new()));
        let provider = Arc::new(MockLlmProvider {
            captured: captured.clone(),
        });
        let runtime = Arc::new(AgentRuntime::new(provider, Arc::new(MockMessageStore)));
        let handler = ChatServiceHandler::new(runtime);

        let request = tonic::Request::new(SendMessageRequest {
            request_id: "test-123".to_string(),
            session_id: "".to_string(),
            agent_id: "".to_string(),
            content: vec![text_block("hello runtime")],
            metadata: Default::default(),
        });

        let response = handler.send_message(request).await.unwrap();
        let resp = response.into_inner();

        assert_eq!(resp.request_id, "test-123");
        assert_eq!(resp.session_id, "session-unknown");

        let requests = captured.lock().expect("lock captured");
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].messages[0].role, "system");
        assert_eq!(requests[0].messages[1].content, "hello runtime");
    }

    #[tokio::test]
    async fn test_send_message_requires_text_content() {
        let captured = Arc::new(Mutex::new(Vec::new()));
        let provider = Arc::new(MockLlmProvider {
            captured: captured.clone(),
        });
        let runtime = Arc::new(AgentRuntime::new(provider, Arc::new(MockMessageStore)));
        let handler = ChatServiceHandler::new(runtime);

        let request = tonic::Request::new(SendMessageRequest {
            request_id: "test-123".to_string(),
            session_id: "".to_string(),
            agent_id: "".to_string(),
            content: vec![],
            metadata: Default::default(),
        });

        let err = handler.send_message(request).await.unwrap_err();
        assert_eq!(err.code(), tonic::Code::InvalidArgument);
    }
}
