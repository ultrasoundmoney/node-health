use node_health::consensus::ConsensusNode;
use node_health::env::ENV_CONFIG;
use node_health::execution_node::ExecutionNode;

#[tokio::test]
#[ignore = "requires live node"]
async fn test_execution_node_peer_count() -> anyhow::Result<()> {
    let execution_node = ExecutionNode::new(ENV_CONFIG.execution_node_url.clone());
    execution_node.peer_count().await?;
    Ok(())
}

#[tokio::test]
#[ignore = "requires live node"]
async fn test_execution_node_is_syncing() -> anyhow::Result<()> {
    let execution_node = ExecutionNode::new(ENV_CONFIG.execution_node_url.clone());
    execution_node.is_syncing().await?;
    Ok(())
}

#[tokio::test]
#[ignore = "requires live node"]
async fn test_execution_node_block_age() -> anyhow::Result<()> {
    let execution_node = ExecutionNode::new(ENV_CONFIG.execution_node_url.clone());
    let age = execution_node.latest_block_age_secs().await?;
    // A synced node should have a block younger than a few minutes.
    assert!(age < 300, "block age {age}s seems too old");
    Ok(())
}

#[tokio::test]
#[ignore = "requires live node"]
async fn test_execution_node_ping_ok() -> anyhow::Result<()> {
    let execution_node = ExecutionNode::new(ENV_CONFIG.execution_node_url.clone());
    execution_node.ping_ok().await?;
    Ok(())
}

#[tokio::test]
#[ignore = "requires live node"]
async fn test_consensus_peer_counts() -> anyhow::Result<()> {
    let consensus = ConsensusNode::new(ENV_CONFIG.beacon_url.clone());
    let peer_counts = consensus.peer_counts().await?;
    dbg!(peer_counts);
    Ok(())
}

#[tokio::test]
#[ignore = "requires live node"]
async fn test_consensus_health() -> anyhow::Result<()> {
    let consensus = ConsensusNode::new(ENV_CONFIG.beacon_url.clone());
    let status = consensus.health().await?;
    assert!(status == 200 || status == 206, "unexpected health status: {status}");
    Ok(())
}

#[tokio::test]
#[ignore = "requires live node"]
async fn test_consensus_ping_ok() -> anyhow::Result<()> {
    let consensus = ConsensusNode::new(ENV_CONFIG.beacon_url.clone());
    let ping_ok = consensus.ping_ok().await?;
    dbg!(ping_ok);
    Ok(())
}
