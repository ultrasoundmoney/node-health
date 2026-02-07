use std::time::{SystemTime, UNIX_EPOCH};

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

    /// Returns `false` when the node reports it is not syncing, `true` otherwise.
    /// `eth_syncing` returns `false` (bool) when not syncing and an object when syncing.
    pub async fn is_syncing(&self) -> anyhow::Result<bool> {
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
        // `eth_syncing` returns `false` when not syncing, an object when syncing.
        match body["result"].as_bool() {
            Some(false) => Ok(false),
            _ => Ok(true),
        }
    }

    /// Fetches the latest block and returns its age in seconds.
    pub async fn latest_block_age_secs(&self) -> anyhow::Result<u64> {
        let body: String =
            json!({ "jsonrpc":"2.0","method":"eth_getBlockByNumber","params":["latest", false],"id":1 })
                .to_string();
        let res = self
            .client
            .post(&self.node_url)
            .header("content-type", "application/json")
            .body(body)
            .send()
            .await?;
        let body: Value = res.json().await?;
        let timestamp_hex = body["result"]["timestamp"]
            .as_str()
            .ok_or(anyhow::anyhow!("block timestamp is not a string"))?
            .trim_start_matches("0x");
        let block_timestamp = u64::from_str_radix(timestamp_hex, 16)?;
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        Ok(now.saturating_sub(block_timestamp))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_ping_ok() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("POST", "/")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"jsonrpc":"2.0","id":1,"result":"net_version_is_usually_a_string"}"#)
            .create_async()
            .await;

        let execution_node = ExecutionNode::new(server.url());
        let ping_ok = execution_node.ping_ok().await.unwrap();

        assert!(ping_ok);
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_peer_count() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("POST", "/")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"jsonrpc":"2.0","id":1,"result":"0x10"}"#)
            .create_async()
            .await;

        let execution_node = ExecutionNode::new(server.url());
        let peer_count = execution_node.peer_count().await.unwrap();

        assert_eq!(peer_count, 16);
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_is_syncing_false() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("POST", "/")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"jsonrpc":"2.0","id":1,"result":false}"#)
            .create_async()
            .await;

        let execution_node = ExecutionNode::new(server.url());
        let syncing = execution_node.is_syncing().await.unwrap();

        assert!(!syncing);
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_is_syncing_true() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("POST", "/")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{"jsonrpc":"2.0","id":1,"result":{"startingBlock":"0x0","currentBlock":"0x10","highestBlock":"0x20"}}"#,
            )
            .create_async()
            .await;

        let execution_node = ExecutionNode::new(server.url());
        let syncing = execution_node.is_syncing().await.unwrap();

        assert!(syncing);
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_latest_block_age_secs() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        // Block from 10 seconds ago
        let timestamp = format!("0x{:x}", now - 10);
        let response_body = format!(
            r#"{{"jsonrpc":"2.0","id":1,"result":{{"timestamp":"{}","number":"0x1"}}}}"#,
            timestamp
        );

        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("POST", "/")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(response_body)
            .create_async()
            .await;

        let execution_node = ExecutionNode::new(server.url());
        let age = execution_node.latest_block_age_secs().await.unwrap();

        // Allow 2 seconds of clock drift in test
        assert!(age >= 9 && age <= 12, "expected ~10s, got {age}s");
        mock.assert_async().await;
    }
}
