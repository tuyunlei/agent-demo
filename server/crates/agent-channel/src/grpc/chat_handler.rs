use std::pin::Pin;

use super::UserId;
use agent_proto::chat_service_server::ChatService;
use agent_proto::{
    ChatEvent, SendMessageRequest, SendMessageResponse, SubmitToolResultRequest,
    SubmitToolResultResponse, SubscribeRequest,
};
use tonic::{Request, Response, Status};

pub struct ChatServiceHandler;

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

        Ok(Response::new(SendMessageResponse {
            request_id: req.request_id,
            session_id: if req.session_id.is_empty() {
                "echo-session-001".to_string()
            } else {
                req.session_id
            },
            user_message_id: "msg-001".to_string(),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_send_message_echo() {
        let handler = ChatServiceHandler;
        let request = tonic::Request::new(SendMessageRequest {
            request_id: "test-123".to_string(),
            session_id: "".to_string(),
            agent_id: "".to_string(),
            content: vec![],
            metadata: Default::default(),
        });

        let response = handler.send_message(request).await.unwrap();
        let resp = response.into_inner();

        assert_eq!(resp.request_id, "test-123");
        assert_eq!(resp.session_id, "echo-session-001");
    }
}
