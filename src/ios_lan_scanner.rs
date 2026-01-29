use std::{
    io,
    net::{Ipv4Addr, UdpSocket},
    sync::Arc,
    time::Duration,
};

use reqwest::StatusCode;
use tokio::sync::Semaphore;

use crate::ios_provider::{DeviceInfoResponse, IosDevice};

#[derive(Clone)]
pub struct IosLanScanner {
    client: reqwest::Client,
    timeout: Duration,
    concurrency: usize,
}

impl IosLanScanner {
    pub fn new(timeout: Duration, concurrency: usize) -> Self {
        Self {
            client: reqwest::Client::new(),
            timeout,
            concurrency,
        }
    }

    pub async fn scan_subnet(&self, subnet_base: Ipv4Addr) -> Vec<IosDevice> {
        let semaphore = Arc::new(Semaphore::new(self.concurrency));
        let mut tasks = Vec::new();

        for host in 2u8..=254u8 {
            let ip = Ipv4Addr::new(
                subnet_base.octets()[0],
                subnet_base.octets()[1],
                subnet_base.octets()[2],
                host,
            );
            let permit = semaphore.clone().acquire_owned().await.unwrap();
            let client = self.client.clone();
            let timeout = self.timeout;
            tasks.push(tokio::spawn(async move {
                let _permit = permit;
                probe_device(client, ip, timeout).await
            }));
        }

        let mut devices = Vec::new();
        for task in tasks {
            if let Ok(Some(device)) = task.await {
                devices.push(device);
            }
        }

        devices
    }
}

pub fn local_ipv4() -> io::Result<Ipv4Addr> {
    let socket = UdpSocket::bind("0.0.0.0:0")?;
    socket.connect("8.8.8.8:80")?;
    match socket.local_addr()? {
        std::net::SocketAddr::V4(addr) => Ok(*addr.ip()),
        std::net::SocketAddr::V6(_) => Err(io::Error::new(io::ErrorKind::Other, "no ipv4")),
    }
}

async fn probe_device(
    client: reqwest::Client,
    ip: Ipv4Addr,
    timeout: Duration,
) -> Option<IosDevice> {
    let base_url = format!("http://{}", ip);
    let ping_url = format!("{}/ping", base_url);

    let ping = client.get(&ping_url).timeout(timeout).send().await.ok()?;

    if ping.status() != StatusCode::OK {
        return None;
    }

    let info_url = format!("{}/deviceinfo", base_url);
    let info = client.get(&info_url).timeout(timeout).send().await.ok()?;

    if info.status() != StatusCode::OK {
        return None;
    }

    let response: DeviceInfoResponse = info.json().await.ok()?;
    if response.code != 0 {
        return None;
    }

    Some(IosDevice::from(response.data))
}
