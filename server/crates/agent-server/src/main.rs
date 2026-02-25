use agent_server::{ServerConfig, run_server};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = ServerConfig::from_env()?;
    println!("agent-server listening on {}", config.listen_addr);
    run_server(config).await
}
