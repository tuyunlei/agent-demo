use std::sync::Arc;

use super::UserId;
use agent_domain::{MessageStore, StoredMessage, StoredSession};
use agent_proto::content_block::Kind;
use agent_proto::session_service_server::SessionService;
use agent_proto::{
    ChatMessage, ContentBlock, CreateSessionRequest, CreateSessionResponse, GetSessionRequest,
    GetSessionResponse, ListSessionMessagesRequest, ListSessionMessagesResponse,
    ListSessionsRequest, ListSessionsResponse, PaginationResponse, Session, TextBlock,
};
use prost_types::Timestamp;
use tonic::{Request, Response, Status};

use super::error::into_status;

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
    async fn create_session(
        &self,
        request: Request<CreateSessionRequest>,
    ) -> Result<Response<CreateSessionResponse>, Status> {
        let user_id = user_id_from_request(&request)
            .ok_or_else(|| Status::unauthenticated("missing user identity"))?;
        let req = request.into_inner();
        let session = self
            .message_store
            .create_session_with_title(&user_id, req.title.trim())
            .await
            .map_err(into_status)?;

        Ok(Response::new(CreateSessionResponse {
            session: Some(to_proto_session(session)),
        }))
    }

    async fn get_session(
        &self,
        request: Request<GetSessionRequest>,
    ) -> Result<Response<GetSessionResponse>, Status> {
        let user_id = user_id_from_request(&request)
            .ok_or_else(|| Status::unauthenticated("missing user identity"))?;
        let req = request.into_inner();
        let session_id = req.session_id.trim();
        if session_id.is_empty() {
            return Err(Status::invalid_argument("session_id is required"));
        }

        let session = self
            .message_store
            .get_session(&user_id, session_id)
            .await
            .map_err(into_status)?
            .ok_or_else(|| Status::not_found("session not found"))?;

        Ok(Response::new(GetSessionResponse {
            session: Some(to_proto_session(session)),
        }))
    }

    async fn list_sessions(
        &self,
        request: Request<ListSessionsRequest>,
    ) -> Result<Response<ListSessionsResponse>, Status> {
        let user_id = user_id_from_request(&request)
            .ok_or_else(|| Status::unauthenticated("missing user identity"))?;
        let sessions = self
            .message_store
            .list_sessions(&user_id)
            .await
            .map_err(into_status)?;

        Ok(Response::new(ListSessionsResponse {
            sessions: sessions.into_iter().map(to_proto_session).collect(),
            pagination: Some(PaginationResponse {
                next_page_token: String::new(),
                total_size: 0,
            }),
        }))
    }

    async fn list_session_messages(
        &self,
        request: Request<ListSessionMessagesRequest>,
    ) -> Result<Response<ListSessionMessagesResponse>, Status> {
        let _user_id = user_id_from_request(&request)
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
            .map_err(into_status)?;

        Ok(Response::new(ListSessionMessagesResponse {
            messages: messages.into_iter().map(to_proto_message).collect(),
            pagination: Some(PaginationResponse {
                next_page_token: String::new(),
                total_size: 0,
            }),
        }))
    }
}

fn user_id_from_request<T>(request: &Request<T>) -> Option<String> {
    request.extensions().get::<UserId>().map(|id| id.0.clone())
}

fn normalize_page_size(page_size: i32) -> i64 {
    if page_size <= 0 {
        50
    } else {
        i64::from(page_size.min(200))
    }
}

fn to_proto_session(session: StoredSession) -> Session {
    Session {
        session_id: session.id,
        user_id: session.user_id,
        agent_id: session.agent_id,
        title: session.title,
        summary: session.summary,
        created_at: Some(to_timestamp(session.created_at)),
        updated_at: Some(to_timestamp(session.updated_at)),
        last_message_at: session.last_message_at.map(to_timestamp),
        archived: session.archived,
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

#[cfg(test)]
#[path = "session_handler_tests.rs"]
mod tests;
