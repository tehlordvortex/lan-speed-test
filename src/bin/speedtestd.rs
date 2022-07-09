use lan_speed_test::{initialize_tracing, start_speed_test_server};
use std::env::args;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    initialize_tracing()?;
    let port = args().skip(1).next();
    let _rocket = start_speed_test_server(port).launch().await?;
    Ok(())
}
