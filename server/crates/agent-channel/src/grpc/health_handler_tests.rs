use super::*;

#[tokio::test]
async fn health_check_returns_configured_version() {
    let handler = HealthServiceHandler::new("1.2.3");

    let response = handler
        .health_check(Request::new(HealthCheckRequest {}))
        .await
        .expect("health check should succeed");

    assert_eq!(response.into_inner().version, "1.2.3");
}
