use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use anyhow::Context;
use axum::{extract::State, response::IntoResponse, routing::get, Router, Server};
use node_health::env::{self, ENV_CONFIG};
use reqwest::StatusCode;
use tokio::sync::Notify;
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

pub async fn serve(is_ready: Arc<AtomicBool>, shutdown_notify: &Notify) {
    let result = {
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

        let socket_addr = format!("{address}:{port}").parse().unwrap();

        Server::bind(&socket_addr)
            .serve(app.into_make_service())
            .with_graceful_shutdown(async {
                shutdown_notify.notified().await;
            })
            .await
            .context("running server")
    };

    match result {
        Ok(_) => info!("server thread exiting"),
        Err(e) => {
            error!(%e, "server thread hit error, exiting");
            shutdown_notify.notify_waiters();
        }
    }
}
