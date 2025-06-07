use node_health::env::ENV_CONFIG;
use node_health::execution_node::ExecutionNode;
use node_health::lighthouse::Lighthouse;

#[tokio::test]
async fn test_execution_node_peer_count() -> anyhow::Result<()> {
    let execution_node = ExecutionNode::new(ENV_CONFIG.execution_node_url.clone());
    execution_node.peer_count().await?;
    Ok(())
}

#[tokio::test]
async fn test_execution_node_sync_status() -> anyhow::Result<()> {
    let execution_node = ExecutionNode::new(ENV_CONFIG.execution_node_url.clone());
    execution_node.syncing().await?;
    Ok(())
}

#[tokio::test]
async fn test_execution_node_ping_ok() -> anyhow::Result<()> {
    let execution_node = ExecutionNode::new(ENV_CONFIG.execution_node_url.clone());
    execution_node.ping_ok().await?;
    Ok(())
}

#[tokio::test]
async fn test_lighthouse_peer_counts() -> anyhow::Result<()> {
    let lighthouse = Lighthouse::new(ENV_CONFIG.beacon_url.clone());
    let peer_counts = lighthouse.peer_counts().await?;
    dbg!(peer_counts);
    Ok(())
}

#[tokio::test]
async fn test_lighthouse_sync_status() -> anyhow::Result<()> {
    let lighthouse = Lighthouse::new(ENV_CONFIG.beacon_url.clone());
    let sync_status = lighthouse.sync_status().await?;
    dbg!(sync_status);
    Ok(())
}

#[tokio::test]
async fn test_lighthouse_ping_ok() -> anyhow::Result<()> {
    let lighthouse = Lighthouse::new(ENV_CONFIG.beacon_url.clone());
    let ping_ok = lighthouse.ping_ok().await?;
    dbg!(ping_ok);
    Ok(())
}
