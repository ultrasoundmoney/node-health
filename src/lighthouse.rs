use reqwest::Client;
use serde::Deserialize;
use tracing::debug;

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
    let res = beacon_client.get(url).send().await;
    match res {
        Ok(res) => Ok(res.status().is_success()),
        Err(e) => {
            debug!("lighthouse ping failed: {}", e);
            Ok(false)
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

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
