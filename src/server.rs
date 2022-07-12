use log::error;
use rocket::{
    data::Limits,
    data::ToByteUnit,
    futures::{FutureExt, Stream},
    response::stream::ByteStream,
    Build, Config, Rocket, Shutdown,
};
use std::net::Ipv4Addr;

use crate::broadcast_service;

pub fn start_speed_test_server(port: Option<String>) -> Rocket<Build> {
    let port = port
        .unwrap_or(String::from("30000"))
        .parse()
        .expect("Port should be numerical");
    if let Err(err) = broadcast_service(port) {
        error!("Failed to start mDNS service, this instance WILL NOT be auto-discoverable: {err}");
    }
    rocket::build()
        .configure(Config {
            port,
            address: Ipv4Addr::new(0, 0, 0, 0).into(),
            limits: Limits::default().limit("bytes", 64.kibibytes()),
            ..Config::default()
        })
        .mount("/", routes![stream_nonsense])
}

#[get("/stream")]
async fn stream_nonsense(shutdown: Shutdown) -> ByteStream![Vec<u8>] {
    let stream = NonsenseStream { shutdown };
    ByteStream(stream)
}

struct NonsenseStream {
    shutdown: Shutdown,
}

impl NonsenseStream {
    fn generate_nonsense() -> Vec<u8> {
        // TODO: Is there a better way to do this? How does this affect memory usage and perf?
        const BUF_SIZE: usize = 1024 * 64; // 64kiB
        vec![0; BUF_SIZE]
    }
}

impl Stream for NonsenseStream {
    type Item = Vec<u8>;
    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        if (&mut self.shutdown).now_or_never().is_some() {
            std::task::Poll::Ready(None)
        } else {
            std::task::Poll::Ready(Some(Self::generate_nonsense()))
        }
    }
}
