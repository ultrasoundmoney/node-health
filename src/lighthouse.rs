use reqwest::Client;
use serde::Deserialize;

use crate::env::ENV_CONFIG;

#[derive(Debug, Deserialize)]
struct SyncingData {
    el_offline: bool,
    is_optimistic: bool,
    is_syncing: bool,
    #[serde(deserialize_with = "deserialize_u64_from_string")]
    sync_distance: u64,
}

#[derive(Debug, Deserialize)]
pub struct Syncing {
    data: SyncingData,
}

impl Syncing {
    pub fn is_syncing(&self) -> bool {
        self.data.is_syncing
    }

    pub fn is_optimistic(&self) -> bool {
        self.data.is_optimistic
    }

    pub fn is_el_offline(&self) -> bool {
        self.data.el_offline
    }

    // Sync distance will be > 0 when a node restarts and is catching up __even if it is not syncing__.
    pub fn sync_distance(&self) -> u64 {
        self.data.sync_distance
    }
}

pub async fn sync_status(beacon_client: &Client) -> anyhow::Result<Syncing> {
    let url = format!("{}/eth/v1/node/syncing", &ENV_CONFIG.beacon_url);
    let res = beacon_client.get(url).send().await?;
    let body: Syncing = res.json().await?;
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

fn deserialize_u64_from_string<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: String = serde::Deserialize::deserialize(deserializer)?;
    s.parse::<u64>().map_err(serde::de::Error::custom)
}

#[derive(Debug, Deserialize)]
struct PeerCountsData {
    #[serde(deserialize_with = "deserialize_u64_from_string")]
    connected: u64,
}

#[derive(Debug, Deserialize)]
pub struct PeerCounts {
    data: PeerCountsData,
}

impl PeerCounts {
    pub fn peer_count(&self) -> u64 {
        self.data.connected
    }
}

pub async fn peer_counts(beacon_client: &Client) -> anyhow::Result<PeerCounts> {
    let url = format!("{}/eth/v1/node/peer_count", &ENV_CONFIG.beacon_url);
    let res = beacon_client.get(url).send().await?;
    let body: PeerCounts = res.json().await?;
    Ok(body)
}

pub async fn ping_ok(beacon_client: &Client) -> anyhow::Result<bool> {
    let url = format!("{}/eth/v1/node/version", &ENV_CONFIG.beacon_url);
    let res = beacon_client.get(url).send().await?;
    Ok(res.status().is_success())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

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

    #[test]
    fn decode_peer_counts() {
        let json = json!({
            "data": {
                "connected": "87",
                "connecting": "0",
                "disconnected": "719",
                "disconnecting": "0"
            }
        });
        let health: super::PeerCounts = serde_json::from_value(json).unwrap();
        assert_eq!(health.data.connected, 87);
    }

    #[test]
    fn decode_syncing() {
        let json = json!({
            "data": {
                "el_offline": false,
                "head_slot": "5478944",
                "is_optimistic": false,
                "is_syncing": false,
                "sync_distance": "0"
            }
        });
        let health: super::Syncing = serde_json::from_value(json).unwrap();
        assert!(!health.is_syncing());
        assert!(!health.is_optimistic());
        assert!(!health.is_el_offline());
    }
}
