use serde::{Deserialize, Serialize};
use std::{net::Ipv4Addr, sync::Arc, time::Duration};
use tokio::{task::JoinHandle, time::sleep};

use crate::{
    ios_lan_scanner::{local_ipv4, IosLanScanner},
    registry::DeviceRegistry,
};

#[derive(Debug, Clone, Deserialize)]
pub struct DeviceInfoResponse {
    pub code: i32,
    pub message: String,
    pub data: DeviceInfoData,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DeviceInfoData {
    pub devsn: String,
    pub zeversion: String,
    pub sysversion: String,
    pub devname: String,
    pub devmac: String,
    pub deviceid: String,
    pub devtype: String,
    pub wifi_ip: String,
    pub ipaddr: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct IosDevice {
    pub id: String,
    pub display_name: String,
    pub wifi_ip: String,
    pub devsn: String,
    pub zeversion: String,
    pub sysversion: String,
    pub devname: String,
    pub devmac: String,
    pub deviceid: String,
    pub devtype: String,
    pub ipaddr: String,
}

impl From<DeviceInfoData> for IosDevice {
    fn from(data: DeviceInfoData) -> Self {
        let display_name = if data.devname.is_empty() {
            data.deviceid.clone()
        } else {
            data.devname.clone()
        };
        Self {
            id: data.deviceid.clone(),
            display_name,
            wifi_ip: data.wifi_ip.clone(),
            devsn: data.devsn,
            zeversion: data.zeversion,
            sysversion: data.sysversion,
            devname: data.devname,
            devmac: data.devmac,
            deviceid: data.deviceid,
            devtype: data.devtype,
            ipaddr: data.ipaddr,
        }
    }
}

pub struct IosProvider {
    registry: Arc<DeviceRegistry>,
    scanner: IosLanScanner,
    interval: Duration,
}

impl IosProvider {
    pub fn new(registry: Arc<DeviceRegistry>, scanner: IosLanScanner, interval: Duration) -> Self {
        Self {
            registry,
            scanner,
            interval,
        }
    }

    pub fn start(self) -> JoinHandle<()> {
        tokio::spawn(async move {
            loop {
                if let Ok(local_ip) = local_ipv4() {
                    let subnet = subnet_base(local_ip);
                    let devices = self.scanner.scan_subnet(subnet).await;
                    self.registry.update_ios_devices(devices).await;
                }
                sleep(self.interval).await;
            }
        })
    }
}

fn subnet_base(ip: Ipv4Addr) -> Ipv4Addr {
    let octets = ip.octets();
    Ipv4Addr::new(octets[0], octets[1], octets[2], 0)
}
