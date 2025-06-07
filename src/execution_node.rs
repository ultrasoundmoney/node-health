use reqwest::Client;
use serde_json::{json, Value};
use tracing::debug;

pub struct ExecutionNode {
    pub node_url: String,
    client: Client,
}

impl ExecutionNode {
    pub fn new(node_url: String) -> Self {
        Self {
            node_url,
            client: Client::new(),
        }
    }

    #[allow(dead_code)]
    pub async fn syncing(&self) -> anyhow::Result<bool> {
        let body: String =
            json!({ "jsonrpc":"2.0","method":"eth_syncing","params":[],"id":1 }).to_string();
        let res = self
            .client
            .post(&self.node_url)
            .header("content-type", "application/json")
            .body(body)
            .send()
            .await?;
        let body: Value = res.json().await?;
        let execution_node_sync_status = body["result"]
            .as_bool()
            .ok_or(anyhow::anyhow!("execution_node_sync_status is not bool"))?;
        Ok(execution_node_sync_status)
    }

    pub async fn peer_count(&self) -> anyhow::Result<u64> {
        let body: String =
            json!({ "jsonrpc":"2.0","method":"net_peerCount","params":[],"id":1 }).to_string();
        let res = self
            .client
            .post(&self.node_url)
            .header("content-type", "application/json")
            .body(body)
            .send()
            .await?;
        let body: Value = res.json().await?;
        let raw_peer_count = body["result"]
            .as_str()
            .ok_or(anyhow::anyhow!("execution_node_peer_count is not string"))?
            .to_string()
            .replace("0x", "");
        let peer_count = u64::from_str_radix(&raw_peer_count, 16)?;
        Ok(peer_count)
    }

    pub async fn ping_ok(&self) -> anyhow::Result<bool> {
        let body: String =
            json!({ "jsonrpc":"2.0","method":"net_version","params":[],"id":1 }).to_string();
        let res = self
            .client
            .post(&self.node_url)
            .header("content-type", "application/json")
            .body(body)
            .send()
            .await;

        match res {
            Ok(res) => Ok(res.status().is_success()),
            Err(e) => {
                debug!("execution_node ping failed: {}", e);
                Ok(false)
            }
        }
    }
}
