use std::pin::Pin;
use std::sync::Arc;

use super::UserId;
use agent_orchestrator::{TurnExecutor, TurnInput};
use agent_proto::chat_service_server::ChatService;
use agent_proto::{
    ChatEvent, SendMessageRequest, SendMessageResponse, SubmitToolResultRequest,
    SubmitToolResultResponse, SubscribeRequest, content_block,
};
use tonic::{Request, Response, Status};

use super::error::into_status;

pub struct ChatServiceHandler {
    runtime: Arc<TurnExecutor>,
}

impl ChatServiceHandler {
    #[must_use]
    pub fn new(runtime: Arc<TurnExecutor>) -> Self {
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

        let user_text = extract_text(&req)?;
        let result = self
            .runtime
            .run_turn(TurnInput {
                user_id,
                session_id: non_empty(&req.session_id).map(ToString::to_string),
                user_message: user_text,
            })
            .await
            .map_err(into_status)?;

        use agent_proto::ContentBlock;
        use agent_proto::TextBlock;
        use agent_proto::content_block::Kind;

        Ok(Response::new(SendMessageResponse {
            request_id: req.request_id,
            session_id: result.session_id,
            user_message_id: "msg-001".to_string(),
            assistant_content: vec![ContentBlock {
                kind: Some(Kind::Text(TextBlock {
                    text: result.assistant_text,
                })),
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

fn non_empty(value: &str) -> Option<&str> {
    if value.trim().is_empty() {
        None
    } else {
        Some(value)
    }
}

#[cfg(test)]
#[path = "chat_handler_tests.rs"]
mod tests;
