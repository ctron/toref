use tokio::select;
use toref::protocol::server::Server;
use toref::runtime::factory::StandardFactory;
use toref::runtime::Runtime;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();

    log::info!("Starting up...");

    let mut factory = StandardFactory::new();
    factory.register_standard_types();
    let runtime = Runtime::new(factory);
    let requests = runtime.requests();

    let server = Server::new("[::]:61500").await?;

    select! {
        r = server.run(requests) => {
            log::info!("Protocol service exited: {r:?}");
        },
        r = runtime.run() => {
            log::info!("Runtime exited: {r:?}");
        },
    }

    Ok(())
}
