use reqwest::Url;
use rocket::{data::ToByteUnit, futures::TryStreamExt};
use std::{
    net::SocketAddr,
    ops::Div,
    sync::{Arc, Mutex},
};
use tokio::{
    process::Command,
    sync::mpsc,
    time::{sleep, Duration, Instant},
};

const CHUNK_LIMITS: usize = 25;
const MILLISECONDS_BETWEEN_CHECKS: u64 = 250;

#[derive(Debug)]
struct SpeedTestState {
    pub total: u64,
    pub total_since_last_check: u64,
    pub last_check: Instant,
    pub speeds: Vec<f64>,
    pub last_speed: f64,
    pub highest_speed: f64,
}

impl Default for SpeedTestState {
    fn default() -> Self {
        Self {
            total: 0,
            total_since_last_check: 0,
            last_check: Instant::now(),
            speeds: vec![],
            last_speed: 0.0,
            highest_speed: 0.0,
        }
    }
}

pub async fn run_speed_test(server_ip: String) -> Result<(), Box<dyn std::error::Error>> {
    let maybe_ip = server_ip.parse::<SocketAddr>();

    let mut url: Url = if let Ok(ip) = maybe_ip {
        Url::parse(&format!("http://{}:{}", ip.ip(), ip.port()))?
    } else {
        server_ip.parse()?
    };
    if url.path() == "/" {
        url.set_path("/stream");
    }
    println!("Connecting to {}...", url);
    let req = reqwest::get(url).await?;
    let body = req.bytes_stream();
    let state = Arc::new(Mutex::new(SpeedTestState::default()));
    let (tx, mut rx) = mpsc::channel::<u64>(1000);

    tokio::spawn({
        let state = state.clone();
        async move {
            let mut received = vec![];
            while let Some(chunk_length) = rx.recv().await {
                received.push(chunk_length);
                if let Ok(mut state) = state.try_lock() {
                    for length in received.drain(..) {
                        state.total += length;
                        state.total_since_last_check += length;
                    }
                }
            }
        }
    });

    tokio::spawn({
        let state = state.clone();
        async move {
            loop {
                sleep(Duration::from_millis(MILLISECONDS_BETWEEN_CHECKS)).await;

                println!(
                    "{}",
                    String::from_utf8_lossy(&Command::new("clear").output().await.unwrap().stdout)
                );

                let mut state = state.lock().unwrap();
                let elapsed_time = state.last_check.elapsed().as_secs_f64();
                state.last_check = Instant::now();

                let speed = state.total_since_last_check as f64 / elapsed_time;
                state.total_since_last_check = 0;
                state.last_speed = speed;
                if speed > state.highest_speed {
                    state.highest_speed = speed;
                }
                state.speeds.push(speed);
                if state.speeds.len() > CHUNK_LIMITS {
                    state.speeds.remove(0);
                }

                if state.speeds.len() < CHUNK_LIMITS {
                    println!("Gathering measurements...");
                } else {
                    let average_speed: f64 = (state.speeds.clone().iter().sum::<f64>()
                        / state.speeds.len() as f64)
                        .as_mebibytes();
                    println!(
                      "Average speed: {:.4}MiB/s, current speed: {:.4}MiB/s, highest_speed: {:.4}MiB/s, total received: {:.4}",
                      average_speed,
                      state.last_speed.as_mebibytes(),
                      state.highest_speed.as_mebibytes(),
                      (state.total / 1.mebibytes()).mebibytes()
                    );
                }
            }
        }
    });

    let _fut = body
        .try_for_each_concurrent(num_cpus::get(), |chunk| {
            let tx = tx.clone();
            async move {
                let chunk_length = chunk.len() as u64;
                let _result = tx.send(chunk_length).await;
                Ok(())
            }
        })
        .await;

    Ok(())
}

trait AsMebibytes<T>
where
    Self: Div<T>,
{
    fn as_mebibytes(&self) -> T;
}

impl AsMebibytes<f64> for f64 {
    fn as_mebibytes(&self) -> Self {
        *self / (1024.0 * 1024.0)
    }
}
