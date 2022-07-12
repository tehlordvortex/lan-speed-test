use reqwest::Url;
use rocket::{data::ToByteUnit, futures::TryStreamExt};
use std::{
    net::SocketAddr,
    ops::Div,
    sync::{Arc, Mutex, MutexGuard},
};
use tokio::{
    net::TcpStream,
    process::Command,
    sync::{mpsc, watch},
    task::JoinHandle,
    time::{sleep, Duration, Instant},
};

use crate::find_service;

const CHUNK_LIMITS: usize = 20;
const MILLISECONDS_BETWEEN_CHECKS: u64 = 100;

#[derive(Debug)]
struct SpeedTestState {
    pub total: u64,
    pub total_since_last_check: u64,
    pub last_check: Instant,
    pub speeds: Vec<f64>,
    pub last_speed: f64,
    pub highest_speed: f64,
    pub server_addr: Url,
}

impl SpeedTestState {
    fn new(server_addr: Url) -> Self {
        Self {
            total: 0,
            total_since_last_check: 0,
            last_check: Instant::now(),
            speeds: vec![],
            last_speed: 0.0,
            highest_speed: 0.0,
            server_addr,
        }
    }
}

struct Terminate {
    terminated: bool,
    notify: watch::Receiver<()>,
}

impl Terminate {
    pub fn new(notify: watch::Receiver<()>) -> Self {
        Self {
            terminated: false,
            notify,
        }
    }

    pub fn has_terminated(&self) -> bool {
        self.terminated
    }

    pub async fn wait_for_signal(&mut self) {
        if self.has_terminated() {
            return;
        }

        let _ = self.notify.changed().await;

        self.terminated = true;
    }
}

async fn try_connect(server_addresses: Vec<SocketAddr>) -> anyhow::Result<String> {
    let socket = TcpStream::connect(&server_addresses[..]).await?;
    Ok(socket.peer_addr()?.to_string())
}

async fn clear_screen() -> anyhow::Result<()> {
    println!(
        "{}",
        String::from_utf8_lossy(&Command::new("clear").output().await?.stdout)
    );
    Ok(())
}

fn print_current_state<'s>(state: &mut MutexGuard<'s, SpeedTestState>) {
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

    println!("Connected to {}", state.server_addr);
    if state.speeds.len() < CHUNK_LIMITS {
        println!("Gathering measurements...");
    } else {
        let average_speed: f64 =
            (state.speeds.clone().iter().sum::<f64>() / state.speeds.len() as f64).as_mebibytes();
        println!(
            "Average speed: {:.4}MiB/s, current speed: {:.4}MiB/s, highest_speed: {:.4}MiB/s, total received: {:.4}",
            average_speed,
            state.last_speed.as_mebibytes(),
            state.highest_speed.as_mebibytes(),
            (state.total / 1.kibibytes()).kibibytes()
        );
    }
}

fn print_final_state<'s>(state: &mut MutexGuard<'s, SpeedTestState>) {
    println!("Results:");
    let average_speed: f64 =
        (state.speeds.clone().iter().sum::<f64>() / state.speeds.len() as f64).as_mebibytes();
    let highest_speed = state.highest_speed.as_mebibytes();
    println!("Average speed: {average_speed:.4}MiB/s");
    println!("Highest speed: {highest_speed:.4}MiB/s");
}

pub async fn run_speed_test(
    server_addr: Option<String>,
    discovery_timeout: Option<u64>,
    duration: u64,
) -> anyhow::Result<()> {
    let server_ip = match server_addr {
        Some(ip) => ip,
        None => try_connect(find_service(discovery_timeout).await?).await?,
    };
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
    let res = reqwest::get(url.clone()).await?;
    let body = res.bytes_stream();
    let state = Arc::new(Mutex::new(SpeedTestState::new(url)));
    let (tx, mut rx) = mpsc::channel::<u64>(1000);
    let (notify_terminate, _) = watch::channel(());

    let receiver_thread = tokio::spawn({
        let state = state.clone();
        let mut terminate = Terminate::new(notify_terminate.subscribe());
        async move {
            let mut received = vec![];
            loop {
                tokio::select! {
                    chunk_length = rx.recv() => {
                        if let Some(chunk_length) = chunk_length {
                            received.push(chunk_length);
                            if let Ok(mut state) = state.try_lock() {
                                for length in received.drain(..) {
                                    state.total += length;
                                    state.total_since_last_check += length;
                                }
                            }
                        }
                    },
                    _ = terminate.wait_for_signal() => {
                        break;
                    }
                }
            }
        }
    });

    let aggregator_thread: JoinHandle<anyhow::Result<()>> = tokio::spawn({
        let state = state.clone();
        let mut terminate = Terminate::new(notify_terminate.subscribe());
        async move {
            loop {
                tokio::select! {
                    _ = sleep(Duration::from_millis(MILLISECONDS_BETWEEN_CHECKS)) => {
                        clear_screen().await?;
                        let mut state = state.lock().unwrap();
                        print_current_state(&mut state);
                    },
                    _ = terminate.wait_for_signal() => {
                        break Ok(());
                    }
                }
            }
        }
    });

    let consumer_threads = tokio::spawn({
        let mut terminate = Terminate::new(notify_terminate.subscribe());
        async move {
            let future = body.try_for_each_concurrent(num_cpus::get(), |chunk| {
                let tx = tx.clone();
                async move {
                    let chunk_length = chunk.len() as u64;
                    let _result = tx.send(chunk_length).await;
                    Ok(())
                }
            });
            tokio::select! {
                _ = future => {},
                _ = terminate.wait_for_signal() => {}
            }
        }
    });

    let timer_thread: JoinHandle<anyhow::Result<()>> = tokio::spawn(async move {
        sleep(Duration::from_secs(duration)).await;
        notify_terminate.send(())?;
        Ok(())
    });

    let _res = tokio::try_join!(
        receiver_thread,
        aggregator_thread,
        consumer_threads,
        timer_thread
    )?;
    clear_screen().await?;
    print_final_state(&mut state.clone().lock().unwrap());

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
