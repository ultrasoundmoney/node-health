use reqwest::Client;

#[tokio::test]
async fn test_geth_peer_count() -> anyhow::Result<()> {
    node_health::geth::peer_count().await?;
    Ok(())
}

#[tokio::test]
async fn test_geth_sync_status() -> anyhow::Result<()> {
    node_health::geth::syncing().await?;
    Ok(())
}

#[tokio::test]
async fn test_lighthouse_peer_counts() -> anyhow::Result<()> {
    let beacon_client = Client::new();
    let peer_counts = node_health::lighthouse::peer_counts(&beacon_client).await?;
    dbg!(peer_counts);
    Ok(())
}

#[tokio::test]
async fn test_lighthouse_sync_status() -> anyhow::Result<()> {
    let beacon_client = Client::new();
    let sync_status = node_health::lighthouse::sync_status(&beacon_client).await?;
    dbg!(sync_status);
    Ok(())
}

#[tokio::test]
async fn test_lighthouse_eth1_syncing() -> anyhow::Result<()> {
    let beacon_client = Client::new();
    let syncing = node_health::lighthouse::eth1_syncing(&beacon_client).await?;
    dbg!(syncing);
    Ok(())
}
