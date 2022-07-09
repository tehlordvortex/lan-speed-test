use tracing_subscriber;
#[macro_use]
extern crate rocket;

mod client;
mod server;

pub use client::run_speed_test;
pub use server::start_speed_test_server;

pub fn initialize_tracing() {
    tracing_subscriber::fmt::init();
}
