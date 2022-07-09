use lan_speed_test::{initialize_tracing, run_speed_test};
use std::env::args;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    initialize_tracing()?;
    let ip_addr = args().skip(1).next();
    run_speed_test(ip_addr).await?;
    Ok(())
}
