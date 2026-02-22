use std::sync::Arc;

use super::UserId;
use agent_domain::{MessageStore, StoreError, StoredMessage};
use agent_proto::content_block::Kind;
use agent_proto::session_service_server::SessionService;
use agent_proto::{
    ChatMessage, ContentBlock, GetSessionRequest, GetSessionResponse, ListSessionMessagesRequest,
    ListSessionMessagesResponse, ListSessionsRequest, ListSessionsResponse, PaginationResponse,
    TextBlock,
};
use prost_types::Timestamp;
use tonic::{Request, Response, Status};

pub struct SessionServiceHandler {
    message_store: Arc<dyn MessageStore + Send + Sync>,
}

impl SessionServiceHandler {
    pub fn new(message_store: Arc<dyn MessageStore + Send + Sync>) -> Self {
        Self { message_store }
    }
}

#[tonic::async_trait]
impl SessionService for SessionServiceHandler {
    async fn get_session(
        &self,
        _request: Request<GetSessionRequest>,
    ) -> Result<Response<GetSessionResponse>, Status> {
        Err(Status::unimplemented("not implemented"))
    }

    async fn list_sessions(
        &self,
        _request: Request<ListSessionsRequest>,
    ) -> Result<Response<ListSessionsResponse>, Status> {
        Err(Status::unimplemented("not implemented"))
    }

    async fn list_session_messages(
        &self,
        request: Request<ListSessionMessagesRequest>,
    ) -> Result<Response<ListSessionMessagesResponse>, Status> {
        let _user_id = request
            .extensions()
            .get::<UserId>()
            .map(|id| id.0.clone())
            .ok_or_else(|| Status::unauthenticated("missing user identity"))?;

        let req = request.into_inner();
        let session_id = req.session_id.trim();
        if session_id.is_empty() {
            return Err(Status::invalid_argument("session_id is required"));
        }

        let limit = req
            .pagination
            .map(|p| normalize_page_size(p.page_size))
            .unwrap_or(50);

        let messages = self
            .message_store
            .get_session_messages(session_id, limit)
            .await
            .map_err(map_store_error)?;

        Ok(Response::new(ListSessionMessagesResponse {
            messages: messages.into_iter().map(to_proto_message).collect(),
            pagination: Some(PaginationResponse {
                next_page_token: String::new(),
                total_size: 0,
            }),
        }))
    }
}

fn normalize_page_size(page_size: i32) -> i64 {
    if page_size <= 0 {
        50
    } else {
        i64::from(page_size.min(200))
    }
}

fn to_proto_message(message: StoredMessage) -> ChatMessage {
    ChatMessage {
        message_id: message.id,
        session_id: message.session_id,
        role: message.role,
        blocks: vec![ContentBlock {
            kind: Some(Kind::Text(TextBlock {
                text: message.content,
            })),
        }],
        created_at: Some(to_timestamp(message.created_at)),
    }
}

fn to_timestamp(seconds: i64) -> Timestamp {
    Timestamp { seconds, nanos: 0 }
}

fn map_store_error(err: StoreError) -> Status {
    match err {
        StoreError::NotFound(msg) => Status::not_found(msg),
        StoreError::Internal(msg) => Status::internal(msg),
    }
}

#[cfg(test)]
#[path = "session_handler_tests.rs"]
mod tests;
