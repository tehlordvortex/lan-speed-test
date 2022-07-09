use rocket::{
    data::Limits, data::ToByteUnit, response::stream::ReaderStream, Build, Config, Rocket,
};
use std::{io, net::Ipv4Addr};
use tokio::fs::File;

pub fn start_speed_test_server(port: Option<String>) -> Rocket<Build> {
    let port = port
        .unwrap_or(String::from("30000"))
        .parse()
        .expect("Port should be numerical");
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
async fn stream_nonsense() -> io::Result<ReaderStream![File]> {
    let dev_zero = File::open("/dev/zero").await?;

    Ok(ReaderStream::one(dev_zero))
}
