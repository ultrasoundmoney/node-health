use reqwest::Client;

#[tokio::test]
async fn test_geth_peer_count() -> anyhow::Result<()> {
    node_health::geth::peer_count().await?;
    Ok(())
}

#[tokio::test]
async fn test_geth_sync_status() -> anyhow::Result<()> {
    node_health::geth::sync_status().await?;
    Ok(())
}

#[tokio::test]
async fn test_lighthouse_ui_health() -> anyhow::Result<()> {
    let client = Client::new();
    let health = node_health::lighthouse::ui_health(client.clone()).await?;
    dbg!(health);
    Ok(())
}

#[tokio::test]
async fn test_lighthouse_eth1_syncing() -> anyhow::Result<()> {
    let client = Client::new();
    let syncing = node_health::lighthouse::eth1_syncing(client.clone()).await?;
    dbg!(syncing);
    Ok(())
}
