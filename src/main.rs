mod server;

use std::{
    sync::{atomic::AtomicBool, Arc},
    time::Duration,
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

    loop {
        let geth_syncing = geth::syncing().await?;
        if geth_syncing {
            info!("geth is syncing, not ready");
            is_ready.store(false, std::sync::atomic::Ordering::Relaxed);
            sleep(Duration::from_secs(4)).await;
            continue;
        } else {
            debug!("geth is not syncing");
        }

        let geth_peer_count = geth::peer_count().await?;
        if geth_peer_count < 10 {
            info!("geth has less than 10 peers, not ready");
            is_ready.store(false, std::sync::atomic::Ordering::Relaxed);
            sleep(Duration::from_secs(4)).await;
            continue;
        } else {
            debug!("geth has more than 10 peers");
        }

        info!("geth is ready");

        let lighthouse_ui_health = node_health::lighthouse::ui_health(&beacon_client).await?;

        if lighthouse_ui_health.is_syncing() {
            info!("lighthouse is syncing, not ready");
            is_ready.store(false, std::sync::atomic::Ordering::Relaxed);
            sleep(Duration::from_secs(4)).await;
            continue;
        } else {
            debug!("lighthouse is not syncing");
        }

        if lighthouse_ui_health.peer_count() < 10 {
            info!("lighthouse has less than 10 peers, not ready");
            is_ready.store(false, std::sync::atomic::Ordering::Relaxed);
            sleep(Duration::from_secs(4)).await;
            continue;
        } else {
            debug!("lighthouse has more than 10 peers");
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
