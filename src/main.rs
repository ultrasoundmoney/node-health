mod server;

use std::{
    sync::{atomic::AtomicBool, Arc},
    time::{Duration, SystemTime},
};

use node_health::{
    consensus::ConsensusNode,
    env::{BLOCK_RECENCY_THRESHOLD_SECS, ENV_CONFIG},
    execution_node::ExecutionNode,
    log,
};
use std::sync::atomic::Ordering;
use tokio::sync::oneshot;
use tokio::{spawn, time::sleep};
use tracing::{debug, info, warn};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    log::init();

    info!("starting node-health");

    let is_ready = Arc::new(AtomicBool::new(false));
    // Shutdown channels: one for server, one for main loop
    let (server_shutdown_tx, server_shutdown_rx) = oneshot::channel::<()>();
    let (main_shutdown_tx, mut main_shutdown_rx) = oneshot::channel::<()>();

    // Start HTTP server with graceful shutdown support
    spawn({
        let is_ready = is_ready.clone();
        async move { server::serve(is_ready, server_shutdown_rx).await }
    });

    // Termination signal listener: SIGTERM/SIGINT (Kubernetes sends SIGTERM)
    spawn({
        let is_ready = is_ready.clone();
        let server_shutdown_tx = server_shutdown_tx;
        let main_shutdown_tx = main_shutdown_tx;
        async move {
            shutdown_signal().await;
            info!("termination signal received; shutting down");
            is_ready.store(false, Ordering::Relaxed);
            let _ = server_shutdown_tx.send(());
            let _ = main_shutdown_tx.send(());
        }
    });

    let execution_node = ExecutionNode::new(ENV_CONFIG.execution_node_url.clone());
    let consensus = ConsensusNode::new(ENV_CONFIG.beacon_url.clone());

    // It can take a long time for nodes to start responding to requests, so we
    // wait until they are reachable before entering the health check loop.
    const MAX_STARTUP_TIME: Duration = Duration::from_secs(60 * 15);
    let start_time = SystemTime::now();
    loop {
        let el_ping_ok = execution_node.ping_ok().await?;
        let cl_ping_ok = consensus.ping_ok().await?;

        if el_ping_ok && cl_ping_ok {
            info!("execution node and consensus node are up");
            break;
        } else {
            debug!(el_ping_ok, cl_ping_ok, "waiting for nodes to come up");
        }

        if start_time.elapsed()? > MAX_STARTUP_TIME {
            anyhow::bail!("execution node and consensus node did not start responding in time");
        }

        debug!("sleeping 4s until next check");
        sleep(Duration::from_secs(4)).await;
    }

    let min_el_peers = ENV_CONFIG.network.min_el_peer_count();
    let min_cl_peers = ENV_CONFIG.network.min_cl_peer_count();

    loop {
        // Run all checks, capturing errors as strings rather than propagating.
        let el_syncing = execution_node
            .is_syncing()
            .await
            .map_err(|e| e.to_string());

        let el_block_age = execution_node
            .latest_block_age_secs()
            .await
            .map_err(|e| e.to_string());

        let el_peers = execution_node
            .peer_count()
            .await
            .map_err(|e| e.to_string());

        let cl_health = consensus.health().await.map_err(|e| e.to_string());

        let cl_peers = consensus
            .peer_counts()
            .await
            .map(|pc| pc.peer_count())
            .map_err(|e| e.to_string());

        // Evaluate each criterion.
        let el_not_syncing = el_syncing.as_ref() == Ok(&false);
        let el_block_fresh = el_block_age
            .as_ref()
            .map(|age| *age < BLOCK_RECENCY_THRESHOLD_SECS)
            .unwrap_or(false);
        let el_peers_ok = el_peers.as_ref().map(|p| *p >= min_el_peers).unwrap_or(false);
        let cl_healthy = cl_health.as_ref() == Ok(&200);
        let cl_peers_ok = cl_peers.as_ref().map(|p| *p >= min_cl_peers).unwrap_or(false);

        let ready = el_not_syncing && el_block_fresh && el_peers_ok && cl_healthy && cl_peers_ok;

        // Format values for logging — show the value or the error.
        let el_syncing_str = match &el_syncing {
            Ok(v) => v.to_string(),
            Err(e) => format!("err: {e}"),
        };
        let el_block_age_str = match &el_block_age {
            Ok(v) => v.to_string(),
            Err(e) => format!("err: {e}"),
        };
        let el_peers_str = match &el_peers {
            Ok(v) => v.to_string(),
            Err(e) => format!("err: {e}"),
        };
        let cl_health_str = match &cl_health {
            Ok(v) => v.to_string(),
            Err(e) => format!("err: {e}"),
        };
        let cl_peers_str = match &cl_peers {
            Ok(v) => v.to_string(),
            Err(e) => format!("err: {e}"),
        };

        info!(
            ready,
            check = "health_cycle",
            el_syncing = %el_syncing_str,
            el_block_age_secs = %el_block_age_str,
            el_peers = %el_peers_str,
            cl_health = %cl_health_str,
            cl_peers = %cl_peers_str,
        );

        if !ready {
            if !el_not_syncing {
                warn!(el_syncing = %el_syncing_str, "EL is syncing or check failed");
            }
            if !el_block_fresh {
                warn!(el_block_age_secs = %el_block_age_str, threshold = BLOCK_RECENCY_THRESHOLD_SECS, "EL block is stale or check failed");
            }
            if !el_peers_ok {
                warn!(el_peers = %el_peers_str, min = min_el_peers, "EL peers below threshold or check failed");
            }
            if !cl_healthy {
                warn!(cl_health = %cl_health_str, "CL reports not healthy or check failed");
            }
            if !cl_peers_ok {
                warn!(cl_peers = %cl_peers_str, min = min_cl_peers, "CL peers below threshold or check failed");
            }
        }

        is_ready.store(ready, Ordering::Relaxed);

        tokio::select! {
            _ = sleep(Duration::from_secs(4)) => {},
            _ = &mut main_shutdown_rx => { break; }
        }
    }
    info!("shutdown complete");
    Ok(())
}

// Cross-platform shutdown signal future.
// - On Unix: waits for SIGTERM or SIGINT
// - Elsewhere: waits for Ctrl-C
async fn shutdown_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};
        let mut term = signal(SignalKind::terminate()).expect("failed to bind SIGTERM");
        let mut int = signal(SignalKind::interrupt()).expect("failed to bind SIGINT");
        tokio::select! {
            _ = term.recv() => {},
            _ = int.recv() => {},
        }
    }
    #[cfg(not(unix))]
    {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to listen for ctrl_c");
    }
}
