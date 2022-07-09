use std::net::{IpAddr, SocketAddr};

use log::info;
use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};
use pnet_datalink as datalink;
use rand::prelude::*;

const SERVICE_TYPE: &str = "_speedtestd._tcp.local.";

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

pub async fn find_service(timeout_in_secs: Option<u32>) -> anyhow::Result<Vec<SocketAddr>> {
    // TODO: Timeout so we don't just keep searching forever?
    let _timeout_in_secs = timeout_in_secs.unwrap_or(60);
    let mdns = ServiceDaemon::new()?;
    println!("Searching for speedtestd on available networks...");
    let receiver = mdns.browse(SERVICE_TYPE)?;
    let addresses = tokio::spawn(async move {
        loop {
            match receiver.recv_async().await {
                Ok(event) => match event {
                    ServiceEvent::ServiceResolved(info) => {
                        println!("Found one: {}", info.get_fullname());
                        let port = info.get_port();
                        let socket_addresses = info
                            .get_addresses()
                            .iter()
                            .map(|ip| SocketAddr::new(IpAddr::V4(*ip), port))
                            .collect();
                        break Ok(socket_addresses);
                    }
                    other_event => {
                        println!("{:#?}", other_event);
                    }
                },
                Err(err) => {
                    break Err(err);
                }
            }
        }
    })
    .await??;

    Ok(addresses)
}
