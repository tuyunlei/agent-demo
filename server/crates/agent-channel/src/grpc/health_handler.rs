use agent_proto::health_service_server::HealthService as HealthServiceTrait;
use agent_proto::{HealthCheckRequest, HealthCheckResponse};
use tonic::{Request, Response, Status};

pub struct HealthServiceHandler {
    version: String,
}

impl HealthServiceHandler {
    #[must_use]
    pub fn new(version: impl Into<String>) -> Self {
        Self {
            version: version.into(),
        }
    }
}

#[tonic::async_trait]
impl HealthServiceTrait for HealthServiceHandler {
    async fn health_check(
        &self,
        _request: Request<HealthCheckRequest>,
    ) -> Result<Response<HealthCheckResponse>, Status> {
        Ok(Response::new(HealthCheckResponse {
            version: self.version.clone(),
        }))
    }
}

#[cfg(test)]
#[path = "health_handler_tests.rs"]
mod tests;
