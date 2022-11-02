use toref::protocol::server::Server;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();

    log::info!("Starting up...");

    let server = Server::new("[::]:61500").await?;
    server.run().await?;

    Ok(())
}
