use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use anyhow::Context;
use axum::{extract::State, http::StatusCode, response::IntoResponse, routing::get, Router};
use node_health::env::{self, ENV_CONFIG};
use tokio::sync::oneshot::Receiver;
use tracing::{error, info};

#[derive(Clone)]
pub struct AppState {
    pub is_ready: Arc<AtomicBool>,
}

async fn is_ready_handler(state: State<AppState>) -> impl IntoResponse {
    if state.is_ready.load(Ordering::Relaxed) {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    }
}

pub async fn serve(is_ready: Arc<AtomicBool>, shutdown_rx: Receiver<()>) {
    let result = async {
        let state = AppState { is_ready };

        let app = Router::new()
            .route("/livez", get(|| async { StatusCode::OK }))
            .route("/readyz", get(is_ready_handler))
            .with_state(state);

        // Developing locally we don't want to expose our server to the world.
        // This also avoids the macOS firewall prompt.
        let address = if ENV_CONFIG.bind_public_interface {
            "0.0.0.0"
        } else {
            "127.0.0.1"
        };

        let port = env::get_env_var("PORT").unwrap_or_else(|| "3004".to_string());

        info!(address, port, "server listening");

        let socket_addr: std::net::SocketAddr =
            format!("{address}:{port}").parse().unwrap();

        let listener = tokio::net::TcpListener::bind(socket_addr)
            .await
            .context("binding TCP listener")?;

        axum::serve(listener, app)
            .with_graceful_shutdown(async move {
                let _ = shutdown_rx.await;
            })
            .await
            .context("running server")
    }
    .await;

    match result {
        Ok(_) => info!("server thread exiting"),
        Err(e) => {
            error!(%e, "server thread hit error, exiting");
            // nothing else to do
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tokio::time::timeout;

    // Verifies the HTTP server respects graceful shutdown notifications and exits promptly.
    #[tokio::test]
    async fn test_server_graceful_shutdown() {
        // Ensure required env vars exist for ENV_CONFIG usage in server
        std::env::set_var("BEACON_URL", "http://localhost:5052");
        std::env::set_var("EXECUTION_NODE_URL", "http://localhost:8545");
        std::env::set_var("NETWORK", "mainnet");
        // Ask OS to assign an ephemeral port
        std::env::set_var("PORT", "0");

        let is_ready = Arc::new(AtomicBool::new(true));
        let (tx, rx) = tokio::sync::oneshot::channel();

        let handle = tokio::spawn({
            let is_ready = is_ready.clone();
            async move { super::serve(is_ready, rx).await }
        });

        // Give the server a brief moment to bind
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Trigger graceful shutdown and assert the task finishes quickly
        let _ = tx.send(());
        let res = timeout(Duration::from_secs(2), handle).await;
        assert!(res.is_ok(), "server did not shut down in time");
    }
}
