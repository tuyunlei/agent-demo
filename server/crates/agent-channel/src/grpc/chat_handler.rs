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
#[path = "chat_handler_tests.rs"]
mod tests;
