use reqwest::Client;

#[tokio::test]
async fn test_execution_node_peer_count() -> anyhow::Result<()> {
    let execution_node_client = Client::new();
    node_health::execution_node::peer_count(&execution_node_client).await?;
    Ok(())
}

#[tokio::test]
async fn test_execution_node_sync_status() -> anyhow::Result<()> {
    let execution_node_client = Client::new();
    node_health::execution_node::syncing(&execution_node_client).await?;
    Ok(())
}

#[tokio::test]
async fn test_execution_node_ping_ok() -> anyhow::Result<()> {
    let execution_node_client = Client::new();
    node_health::execution_node::ping_ok(&execution_node_client).await?;
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
async fn test_lighthouse_ping_ok() -> anyhow::Result<()> {
    let beacon_client = Client::new();
    let ping_ok = node_health::lighthouse::ping_ok(&beacon_client).await?;
    dbg!(ping_ok);
    Ok(())
}
