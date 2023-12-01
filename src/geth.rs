use serde_json::{json, Value};

use crate::env::ENV_CONFIG;

#[allow(dead_code)]
pub async fn syncing() -> anyhow::Result<bool> {
    let client = reqwest::Client::new();
    let body: String =
        json!({ "jsonrpc":"2.0","method":"eth_syncing","params":[],"id":1 }).to_string();
    let res = client
        .post(&ENV_CONFIG.geth_url)
        .header("content-type", "application/json")
        .body(body)
        .send()
        .await?;
    let body: Value = res.json().await?;
    let geth_sync_status = body["result"]
        .as_bool()
        .ok_or(anyhow::anyhow!("geth_sync_status is not bool"))?;
    Ok(geth_sync_status)
}

pub async fn peer_count() -> anyhow::Result<u64> {
    let client = reqwest::Client::new();
    let body: String =
        json!({ "jsonrpc":"2.0","method":"net_peerCount","params":[],"id":1 }).to_string();
    let res = client
        .post(&ENV_CONFIG.geth_url)
        .header("content-type", "application/json")
        .body(body)
        .send()
        .await?;
    let body: Value = res.json().await?;
    let raw_peer_count = body["result"]
        .as_str()
        .ok_or(anyhow::anyhow!("geth_peer_count is not string"))?
        .to_string()
        .replace("0x", "");
    let peer_count = u64::from_str_radix(&raw_peer_count, 16)?;
    Ok(peer_count)
}
