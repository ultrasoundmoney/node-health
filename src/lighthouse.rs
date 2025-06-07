use reqwest::Client;
use serde::Deserialize;
use tracing::debug;

pub struct Lighthouse {
    pub node_url: String,
    client: Client,
}

impl Lighthouse {
    pub fn new(node_url: String) -> Self {
        Self {
            node_url,
            client: Client::new(),
        }
    }

    pub async fn sync_status(&self) -> anyhow::Result<Syncing> {
        let url = format!("{}/eth/v1/node/syncing", &self.node_url);
        let res = self.client.get(url).send().await?;
        let body: Syncing = res.json().await?;
        Ok(body)
    }

    pub async fn peer_counts(&self) -> anyhow::Result<PeerCounts> {
        let url = format!("{}/eth/v1/node/peer_count", &self.node_url);
        let res = self.client.get(url).send().await?;
        let body: PeerCounts = res.json().await?;
        Ok(body)
    }

    pub async fn ping_ok(&self) -> anyhow::Result<bool> {
        let url = format!("{}/eth/v1/node/version", &self.node_url);
        let res = self.client.get(url).send().await;
        match res {
            Ok(res) => Ok(res.status().is_success()),
            Err(e) => {
                debug!("lighthouse ping failed: {}", e);
                Ok(false)
            }
        }
    }
}

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

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::Lighthouse;

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

    #[tokio::test]
    async fn test_ping_ok() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("GET", "/eth/v1/node/version")
            .with_status(200)
            .create_async()
            .await;

        let lighthouse = Lighthouse::new(server.url());
        let ping_ok = lighthouse.ping_ok().await.unwrap();

        assert!(ping_ok);
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_peer_counts() {
        let mut server = mockito::Server::new_async().await;
        let peer_count_response = json!({
            "data": {
                "connected": "87",
                "connecting": "0",
                "disconnected": "719",
                "disconnecting": "0"
            }
        });
        let mock = server
            .mock("GET", "/eth/v1/node/peer_count")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(serde_json::to_string(&peer_count_response).unwrap())
            .create_async()
            .await;

        let lighthouse = Lighthouse::new(server.url());
        let peer_counts = lighthouse.peer_counts().await.unwrap();

        assert_eq!(peer_counts.peer_count(), 87);
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_sync_status() {
        let mut server = mockito::Server::new_async().await;
        let sync_status_response = json!({
            "data": {
                "el_offline": false,
                "head_slot": "5478944",
                "is_optimistic": false,
                "is_syncing": false,
                "sync_distance": "0"
            }
        });
        let mock = server
            .mock("GET", "/eth/v1/node/syncing")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(serde_json::to_string(&sync_status_response).unwrap())
            .create_async()
            .await;

        let lighthouse = Lighthouse::new(server.url());
        let sync_status = lighthouse.sync_status().await.unwrap();

        assert!(!sync_status.is_syncing());
        mock.assert_async().await;
    }
}
