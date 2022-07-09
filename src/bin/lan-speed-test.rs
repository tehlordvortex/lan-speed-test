use lan_speed_test::{initialize_tracing, run_speed_test};
use std::env::args;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    initialize_tracing();
    let ip_addr = args()
        .skip(1)
        .next()
        .expect("Either the URL or IP address of the server.");
    run_speed_test(ip_addr).await
}
