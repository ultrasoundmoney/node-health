mod server;

use std::{
    sync::{atomic::AtomicBool, Arc},
    time::{Duration, SystemTime},
};

use node_health::{geth, log};
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
    let geth_client = reqwest::Client::new();

    // It can take a long long time for the geth and lighthouse nodes to start responding to
    // requests, so we wait until they are ready before we start the server.
    const MAX_STARTUP_TIME: Duration = Duration::from_secs(60 * 2);
    let start_time = SystemTime::now();
    loop {
        let geth_ping_ok = geth::ping_ok(&geth_client).await?;
        let lighthouse_ping_ok = node_health::lighthouse::ping_ok(&beacon_client).await?;

        if geth_ping_ok && lighthouse_ping_ok {
            info!("geth and lighthouse are up");
            break;
        } else {
            debug!(
                "geth_ping_ok: {}, lighthouse_ping_ok: {}",
                geth_ping_ok, lighthouse_ping_ok
            );
        }

        if start_time.elapsed()? > MAX_STARTUP_TIME {
            anyhow::bail!("geth and lighthouse did not start responding in time");
        }

        debug!("sleeping 4s until next check");
        sleep(Duration::from_secs(4)).await;
    }

    loop {
        let geth_syncing = geth::syncing(&geth_client).await?;
        if geth_syncing {
            info!("geth is syncing, not ready");
            is_ready.store(false, std::sync::atomic::Ordering::Relaxed);
            sleep(Duration::from_secs(4)).await;
            continue;
        } else {
            debug!("geth is not syncing");
        }

        let geth_peer_count = geth::peer_count(&geth_client).await?;
        if geth_peer_count < 10 {
            info!(geth_peer_count, "geth has less than 10 peers, not ready");
            is_ready.store(false, std::sync::atomic::Ordering::Relaxed);
            sleep(Duration::from_secs(4)).await;
            continue;
        } else {
            debug!("geth has more than 10 peers");
        }

        info!("geth is ready");

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

        let lighthouse_eth1_syncing = node_health::lighthouse::eth1_syncing(&beacon_client).await?;

        if lighthouse_eth1_syncing.eth1_is_syncing() {
            info!("lighthouse thinks eth1 is syncing, not ready");
            is_ready.store(false, std::sync::atomic::Ordering::Relaxed);
            sleep(Duration::from_secs(4)).await;
            continue;
        } else {
            debug!("lighthouse thinks eth1 is not syncing");
        }

        if !lighthouse_eth1_syncing.is_cached_and_ready() {
            info!("lighthouse cache is not ready");
            is_ready.store(false, std::sync::atomic::Ordering::Relaxed);
            sleep(Duration::from_secs(4)).await;
            continue;
        } else {
            debug!("lighthouse cache is ready");
        }

        info!("lighthouse is ready");

        info!("beacon node is ready for traffic");
        is_ready.store(true, std::sync::atomic::Ordering::Relaxed);

        debug!("sleeping 4s until next check");
        sleep(Duration::from_secs(4)).await;
    }
}
