use std::sync::{Arc, Mutex};

use anyhow::Result;
use serde::{Deserialize, Serialize};
use sysinfo::{CpuRefreshKind, MemoryRefreshKind, Networks, RefreshKind, System};
use tokio::task;

fn round_to(num: f32, places: i32) -> f32 {
    let factor = 10_f32.powi(places);
    (num * factor).round() / factor
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HardwareInfo {
    cpu: u8,
    mem: u8,
    net: f32,
}

pub struct Hardware {
    system: Arc<Mutex<System>>,
    networks: Arc<Mutex<Networks>>,
}

impl Hardware {
    pub fn new() -> Hardware {
        let mut system = System::new_with_specifics(
            RefreshKind::nothing()
                .with_cpu(CpuRefreshKind::nothing().with_cpu_usage())
                .with_memory(MemoryRefreshKind::nothing().with_ram()),
        );
        let networks = Networks::new_with_refreshed_list();
        system.refresh_all();
        Hardware {
            system: Arc::new(Mutex::new(system)),
            networks: Arc::new(Mutex::new(networks)),
        }
    }

    pub async fn refresh(&mut self) -> Result<()> {
        let system = Arc::clone(&self.system);
        let networks = Arc::clone(&self.networks);
        task::spawn_blocking(move || {
            let mut system = system.lock().unwrap();
            let mut networks = networks.lock().unwrap();
            system.refresh_all();
            networks.refresh(true);
        })
        .await?;

        Ok(())
    }

    pub async fn get(&mut self) -> Result<HardwareInfo> {
        let system = Arc::clone(&self.system);
        let networks = Arc::clone(&self.networks);

        self.refresh().await?;
        let system = system.lock().unwrap();
        let networks = networks.lock().unwrap();

        let cpu = (system.global_cpu_usage() as u8) / system.cpus().len() as u8;
        let mem = ((system.used_memory() as f32 / system.total_memory() as f32) * 100.0) as u8;
        let net: f32 = networks
            .iter()
            .map(|(_, v)| v.received() + v.transmitted())
            .sum::<u64>() as f32;

        let hardware_info = HardwareInfo {
            cpu,
            mem,
            net: round_to(net / 1024.0, 2),
        };

        Ok(hardware_info)
    }
}
