use reqwest::Client;
use serde::Deserialize;
use serde_json::Value;

use crate::env::ENV_CONFIG;

#[derive(Debug, Deserialize, PartialEq)]
struct SyncingFinalized {
    #[serde(rename = "SyncingFinalized")]
    syncing_finalized: Value,
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(untagged)]
enum SyncState {
    StringStatus(String),
    Syncing(SyncingFinalized),
}

#[derive(Debug, Deserialize)]
struct HealthData {
    connected_peers: u64,
    sync_state: SyncState,
}

#[derive(Debug, Deserialize)]
pub struct UiHealth {
    data: HealthData,
}

impl UiHealth {
    pub fn is_syncing(&self) -> bool {
        let synced = match &self.data.sync_state {
            SyncState::StringStatus(str) => str == "Synced",
            _ => false,
        };
        !synced
    }

    pub fn peer_count(&self) -> u64 {
        self.data.connected_peers
    }
}

pub async fn ui_health(beacon_client: &Client) -> anyhow::Result<UiHealth> {
    let url = format!("{}/lighthouse/ui/health", &ENV_CONFIG.beacon_url);
    let res = beacon_client.get(url).send().await?;
    let body: UiHealth = res.json().await?;
    Ok(body)
}

#[derive(Debug, Deserialize)]
struct Eth1SyncingData {
    eth1_node_sync_status_percentage: f64,
    lighthouse_is_cached_and_ready: bool,
}

#[derive(Debug, Deserialize)]
pub struct Eth1Syncing {
    data: Eth1SyncingData,
}

impl Eth1Syncing {
    pub fn eth1_is_syncing(&self) -> bool {
        self.data.eth1_node_sync_status_percentage < 100.0
    }

    pub fn is_cached_and_ready(&self) -> bool {
        self.data.lighthouse_is_cached_and_ready
    }
}

pub async fn eth1_syncing(beacon_client: &Client) -> anyhow::Result<Eth1Syncing> {
    let url = format!("{}/lighthouse/eth1/syncing", &ENV_CONFIG.beacon_url);
    let res = beacon_client.get(url).send().await?;
    let body: Eth1Syncing = res.json().await?;
    Ok(body)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn decode_lighthouse_health_syncing() {
        let json = json!({
            "data": {
                "app_uptime": 761,
                "connected_peers": 87,
                "cpu_cores": 6,
                "cpu_threads": 12,
                "disk_bytes_free": 3228685,
                "disk_bytes_total": 1886832,
                "free_memory": 41908879,
                "global_cpu_frequency": 3.4,
                "host_name": "supply-delta-geth",
                "kernel_version": "5.15.0-83-generic",
                "nat_open": true,
                "network_bytes_total_received": 499582704,
                "network_bytes_total_transmit": 270064843,
                "network_name": "enp3s0f0",
                "os_version": "Linux 22.04 Ubuntu",
                "sync_state": {
                    "SyncingFinalized": {
                       "start_slot": "5478848",
                       "target_slot": "5478944"
                    },
                },
                "sys_loadavg_1": 4.78,
                "sys_loadavg_15": 7.6,
                "sys_loadavg_5": 8.22,
                "system_name": "Ubuntu",
                "system_uptime": 6837520,
                "total_memory": 3355867,
                "used_memory": 117984
            }
        });

        let health: super::UiHealth = serde_json::from_value(json).unwrap();
        assert_eq!(health.data.connected_peers, 87);
        assert_eq!(
            health.data.sync_state,
            SyncState::Syncing(SyncingFinalized {
                syncing_finalized: json!({
                    "start_slot": "5478848",
                    "target_slot": "5478944"
                })
            })
        );
    }

    #[test]
    fn decode_lighthouse_health_synced() {
        let json = json!({
            "data": {
                "app_uptime": 761,
                "connected_peers": 87,
                "cpu_cores": 6,
                "cpu_threads": 12,
                "disk_bytes_free": 3228685,
                "disk_bytes_total": 1886832,
                "free_memory": 41908879,
                "global_cpu_frequency": 3.4,
                "host_name": "supply-delta-geth",
                "kernel_version": "5.15.0-83-generic",
                "nat_open": true,
                "network_bytes_total_received": 499582704,
                "network_bytes_total_transmit": 270064843,
                "network_name": "enp3s0f0",
                "os_version": "Linux 22.04 Ubuntu",
                "sync_state": "Synced",
                "sys_loadavg_1": 4.78,
                "sys_loadavg_15": 7.6,
                "sys_loadavg_5": 8.22,
                "system_name": "Ubuntu",
                "system_uptime": 6837520,
                "total_memory": 3355867,
                "used_memory": 117984
            }
        });

        let health: super::UiHealth = serde_json::from_value(json).unwrap();
        assert_eq!(health.data.connected_peers, 87);
        assert_eq!(
            health.data.sync_state,
            SyncState::StringStatus("Synced".to_string())
        );
    }

    #[test]
    fn decode_eth1_synced() {
        let json = json!({
            "data": {
                "eth1_node_sync_status_percentage": 100.0,
                "head_block_number": 18692195,
                "head_block_timestamp": 1701441371,
                "latest_cached_block_number": 18690654,
                "latest_cached_block_timestamp": 1701422687,
                "lighthouse_is_cached_and_ready": true,
                "voting_target_timestamp": 1701388375
            }
        });
        let health: super::Eth1Syncing = serde_json::from_value(json).unwrap();
        assert!(!health.eth1_is_syncing());
        assert!(health.is_cached_and_ready());
    }

    #[test]
    fn decode_eth1_syncing() {
        let json = json!({
            "data": {
                "eth1_node_sync_status_percentage": 99.99991975188567,
                "head_block_number": 18692195,
                "head_block_timestamp": 1701441371,
                "latest_cached_block_number": 18690654,
                "latest_cached_block_timestamp": 1701422687,
                "lighthouse_is_cached_and_ready": true,
                "voting_target_timestamp": 1701388375
            }
        });
        let health: super::Eth1Syncing = serde_json::from_value(json).unwrap();
        assert!(health.eth1_is_syncing());
    }
}
