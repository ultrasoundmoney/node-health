use reqwest::Client;
use serde::Deserialize;
use tracing::debug;

pub struct ConsensusNode {
    pub node_url: String,
    client: Client,
}

impl ConsensusNode {
    pub fn new(node_url: String) -> Self {
        Self {
            node_url,
            client: Client::new(),
        }
    }

    /// Calls the standard Beacon API health endpoint.
    /// Returns the HTTP status code: 200=ready, 206=syncing, 503=not ready.
    pub async fn health(&self) -> anyhow::Result<u16> {
        let url = format!("{}/eth/v1/node/health", &self.node_url);
        let res = self.client.get(url).send().await?;
        Ok(res.status().as_u16())
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
                debug!("consensus node ping failed: {}", e);
                Ok(false)
            }
        }
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

    use super::ConsensusNode;

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

    #[tokio::test]
    async fn test_ping_ok() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("GET", "/eth/v1/node/version")
            .with_status(200)
            .create_async()
            .await;

        let consensus = ConsensusNode::new(server.url());
        let ping_ok = consensus.ping_ok().await.unwrap();

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

        let consensus = ConsensusNode::new(server.url());
        let peer_counts = consensus.peer_counts().await.unwrap();

        assert_eq!(peer_counts.peer_count(), 87);
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_health_ready() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("GET", "/eth/v1/node/health")
            .with_status(200)
            .create_async()
            .await;

        let consensus = ConsensusNode::new(server.url());
        let status = consensus.health().await.unwrap();

        assert_eq!(status, 200);
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_health_syncing() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("GET", "/eth/v1/node/health")
            .with_status(206)
            .create_async()
            .await;

        let consensus = ConsensusNode::new(server.url());
        let status = consensus.health().await.unwrap();

        assert_eq!(status, 206);
        mock.assert_async().await;
    }
}
