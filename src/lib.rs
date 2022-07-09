use simplelog::{ColorChoice, Config, LevelFilter, TermLogger, TerminalMode};
#[macro_use]
extern crate rocket;

mod client;
mod server;
mod service_discovery;

pub use client::run_speed_test;
pub use server::start_speed_test_server;
pub use service_discovery::{broadcast_service, find_service};

pub fn initialize_tracing() -> anyhow::Result<()> {
    TermLogger::init(
        LevelFilter::Info,
        Config::default(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    )?;
    Ok(())
}
