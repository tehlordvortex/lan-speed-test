use clap::Parser;
use lan_speed_test::{initialize_tracing, run_speed_test};

#[derive(Debug, Parser)]
#[clap(author, version, about, long_about = None)]
struct CliArgs {
    /// Timeout in seconds for finding a server
    #[clap(short, long, value_parser, default_value_t = 60)]
    timeout: u64,
    /// Duration in seconds for the test.
    #[clap(short, long, value_parser, default_value_t = 30)]
    duration: u64,
    /// The IP:PORT address of the server or a URL (e.g. http://my-cloud.provider.somewhere).
    /// If a path segement is not included, it defaults to /stream
    #[clap(value_parser)]
    server_addr: Option<String>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    initialize_tracing()?;
    let args = CliArgs::parse();
    run_speed_test(args.server_addr, Some(args.timeout), args.duration).await?;
    Ok(())
}
