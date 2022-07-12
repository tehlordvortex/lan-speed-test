use std::net::{IpAddr, SocketAddr};

use log::info;
use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};
use pnet_datalink as datalink;
use rand::prelude::*;
use tokio::time::{sleep, Duration};

const SERVICE_TYPE: &str = "_speedtestd._tcp.local.";

#[derive(Debug)]
struct DiscoveryTimeoutError {}
impl std::error::Error for DiscoveryTimeoutError {}
impl std::fmt::Display for DiscoveryTimeoutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Timeout while trying to discover speed test servers.")
    }
}

pub fn broadcast_service(port: u16) -> anyhow::Result<()> {
    let mdns = ServiceDaemon::new()?;
    let host_name = {
        let name = hostname::get()?;
        name.to_string_lossy().to_string()
    };
    let instance_name = rand::thread_rng().gen_range(0..255).to_string();

    let interfaces = datalink::interfaces();
    let ips: String = interfaces
        .iter()
        .filter(|interface| interface.is_up() && !interface.is_loopback() && interface.is_running())
        .flat_map(|interface| interface.ips.clone())
        .filter(|ip_network| ip_network.is_ipv4())
        .map(|ip_network| match ip_network.ip() {
            IpAddr::V4(ipv4) => ipv4,
            _ => unreachable!(),
        })
        .map(|ip| ip.to_string())
        .collect::<Vec<String>>()
        .join(",");

    info!("Broadcasting availability as {instance_name}, hostname: {host_name}, on {ips}");
    let service = ServiceInfo::new(SERVICE_TYPE, &instance_name, &host_name, ips, port, None)?;

    mdns.register(service)?;
    Ok(())
}

pub async fn find_service(timeout_in_secs: Option<u64>) -> anyhow::Result<Vec<SocketAddr>> {
    let timeout_in_secs = timeout_in_secs.unwrap_or(60);
    let mdns = ServiceDaemon::new()?;
    println!("Searching for speedtestd on available networks...");
    let receiver = mdns.browse(SERVICE_TYPE)?;
    let find_addresses = tokio::spawn(async move {
        loop {
            match receiver.recv_async().await {
                Ok(event) => match event {
                    ServiceEvent::ServiceResolved(info) => {
                        info!("Found an instance: {}", info.get_fullname());
                        let port = info.get_port();
                        let socket_addresses = info
                            .get_addresses()
                            .iter()
                            .map(|ip| SocketAddr::new(IpAddr::V4(*ip), port))
                            .collect();
                        break Ok(socket_addresses);
                    }
                    _ => {}
                },
                Err(err) => {
                    break Err(err.into());
                }
            }
        }
    });
    let timeout = tokio::spawn(async move {
        sleep(Duration::from_secs(timeout_in_secs)).await;
        Err((DiscoveryTimeoutError {}).into())
    });
    let maybe_addresses = tokio::select! {
      addresses = find_addresses => {
        addresses?
      }
      err = timeout => {
        err?
      }
    };

    maybe_addresses
}
