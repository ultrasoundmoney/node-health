mod server;

use std::{
    sync::{atomic::AtomicBool, Arc},
    time::{Duration, SystemTime},
};

use node_health::{
    env::{Network, ENV_CONFIG},
    execution_node, log,
};
use tokio::{spawn, sync::Notify, time::sleep};
use tracing::{debug, info};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    log::init();

    info!("starting node-health");

    let shutdown_notify = Notify::new();

    let is_ready = Arc::new(AtomicBool::new(false));

    spawn({
        let is_ready = is_ready.clone();
        async move { server::serve(is_ready, &shutdown_notify).await }
    });

    let beacon_client = reqwest::Client::new();
    let execution_node_client = reqwest::Client::new();

    // It can take a long long time for the execution_node and lighthouse nodes to start responding to
    // requests, so we wait until they are ready before we start the server.
    const MAX_STARTUP_TIME: Duration = Duration::from_secs(60 * 15);
    let start_time = SystemTime::now();
    loop {
        let execution_node_ping_ok = execution_node::ping_ok(&execution_node_client).await?;
        let lighthouse_ping_ok = node_health::lighthouse::ping_ok(&beacon_client).await?;

        if execution_node_ping_ok && lighthouse_ping_ok {
            info!("execution_node and lighthouse are up");
            break;
        } else {
            debug!(
                "execution_node_ping_ok: {}, lighthouse_ping_ok: {}",
                execution_node_ping_ok, lighthouse_ping_ok
            );
        }

        if start_time.elapsed()? > MAX_STARTUP_TIME {
            anyhow::bail!("execution_node and lighthouse did not start responding in time");
        }

        debug!("sleeping 4s until next check");
        sleep(Duration::from_secs(4)).await;
    }

    loop {
        let execution_node_syncing = execution_node::syncing(&execution_node_client).await;
        match execution_node_syncing {
            Ok(execution_node_syncing) => {
                if execution_node_syncing {
                    info!("execution_node is syncing");
                    is_ready.store(false, std::sync::atomic::Ordering::Relaxed);
                    sleep(Duration::from_secs(4)).await;
                    continue;
                } else {
                    debug!("execution_node is not syncing");
                }
            }
            Err(e) => {
                debug!("execution_node sync check failed: {}, not ready", e);
                is_ready.store(false, std::sync::atomic::Ordering::Relaxed);
                sleep(Duration::from_secs(4)).await;
                continue;
            }
        }

        // Peer check doesn't work on goerli, so we skip it.
        if ENV_CONFIG.network == Network::Goerli {
            debug!("goerli network, skipping execution_node peer count check");
        } else {
            let min_peer_count = if ENV_CONFIG.network == Network::Mainnet {
                5
            } else {
                2
            };
            let execution_node_peer_count =
                execution_node::peer_count(&execution_node_client).await;
            match execution_node_peer_count {
                Ok(execution_node_peer_count) => {
                    if execution_node_peer_count < min_peer_count {
                        info!(
                            execution_node_peer_count,
                            "execution_node has less than {min_peer_count} peers, not ready"
                        );
                        is_ready.store(false, std::sync::atomic::Ordering::Relaxed);
                        sleep(Duration::from_secs(4)).await;
                        continue;
                    } else {
                        debug!("execution_node has more than {min_peer_count} peers");
                    }
                }
                Err(e) => {
                    debug!("execution_node peer count check failed: {}, not ready", e);
                    is_ready.store(false, std::sync::atomic::Ordering::Relaxed);
                    sleep(Duration::from_secs(4)).await;
                    continue;
                }
            }
        }

        info!("execution_node is ready");

        let lighthouse_peer_counts = node_health::lighthouse::peer_counts(&beacon_client).await?;
        let lighthouse_peer_count = lighthouse_peer_counts.peer_count();
        if lighthouse_peer_count < 10 {
            info!(
                lighthouse_peer_count,
                "lighthouse has less than 10 peers, not ready"
            );
            is_ready.store(false, std::sync::atomic::Ordering::Relaxed);
            sleep(Duration::from_secs(4)).await;
            continue;
        } else {
            debug!("lighthouse has more than 10 peers");
        }

        let lighthouse_sync_status = node_health::lighthouse::sync_status(&beacon_client).await?;

        if lighthouse_sync_status.is_syncing() {
            info!("lighthouse is syncing, not ready");
            is_ready.store(false, std::sync::atomic::Ordering::Relaxed);
            sleep(Duration::from_secs(4)).await;
            continue;
        } else {
            debug!("lighthouse is not syncing");
        }

        if lighthouse_sync_status.is_optimistic() {
            info!("lighthouse sync is optimistic, not ready");
            is_ready.store(false, std::sync::atomic::Ordering::Relaxed);
            sleep(Duration::from_secs(4)).await;
            continue;
        } else {
            debug!("lighthouse is not optimistic");
        }

        if lighthouse_sync_status.is_el_offline() {
            info!("lighthouse says is el offline, not ready");
            is_ready.store(false, std::sync::atomic::Ordering::Relaxed);
            sleep(Duration::from_secs(4)).await;
            continue;
        } else {
            debug!("lighthouse is not el offline");
        }

        let sync_distance = lighthouse_sync_status.sync_distance();
        // We allow to be one slot behind, this naturally happens all the time.
        if sync_distance > 1 {
            info!(
                sync_distance,
                "lighthouse sync distance is greater than 0, not ready"
            );
            is_ready.store(false, std::sync::atomic::Ordering::Relaxed);
            sleep(Duration::from_secs(4)).await;
            continue;
        } else {
            debug!("lighthouse sync distance is 0");
        }

        info!("lighthouse is ready");

        info!("beacon node is ready for traffic");
        is_ready.store(true, std::sync::atomic::Ordering::Relaxed);

        debug!("sleeping 4s until next check");
        sleep(Duration::from_secs(4)).await;
    }
}
