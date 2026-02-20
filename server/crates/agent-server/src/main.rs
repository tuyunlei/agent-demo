use agent_channel::grpc::ChatServiceHandler;
use agent_proto::chat_service_server::ChatServiceServer;
use tonic::transport::Server;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "[::1]:50051".parse()?;
    println!("agent-server listening on {}", addr);

    Server::builder()
        .add_service(ChatServiceServer::new(ChatServiceHandler))
        .serve(addr)
        .await?;

    Ok(())
}
