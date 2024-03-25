use crate::{
    config::Config,
    hydra::{
        client::HydraClient,
        store::{generate_machines_file, wake_builders, watch_job_queue, Store},
    },
    middleware::allowed_ips,
};
use anyhow::Context;
use axum::{routing::get, Router};

use hydra_sentinel::shutdown_signal;
use listenfd::ListenFd;
use secrecy::SecretString;
use std::{future::IntoFuture, net::SocketAddr, sync::Arc, time::Duration};
use tokio::net::TcpListener;
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::{DefaultMakeSpan, TraceLayer};

mod config;
mod error;
mod github;
mod hydra;
mod middleware;
mod model;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = hydra_sentinel::init::<Config>(&format!("{}=DEBUG", module_path!()))?;

    // TODO: Optional; disable webhooks without
    let github_webhook_secret = std::fs::read_to_string(&config.github_webhook_secret_file)
        .map(SecretString::from)
        .context("Failed to read github webhook secret")?;

    let hydra_client = HydraClient::new(config.hydra_base_url);

    // build our application with some routes
    let store = Arc::new(Store::new(config.heartbeat_timeout, config.build_machines));
    let app = Router::new()
        .route("/webhook", github::webhook::handler(github_webhook_secret))
        .with_state(hydra_client.clone())
        .route(
            "/ws",
            get(hydra::websocket::connect).route_layer(axum::middleware::from_fn_with_state(
                config.allowed_ips,
                allowed_ips,
            )),
        )
        .with_state(store.clone())
        .layer((
            TraceLayer::new_for_http().make_span_with(DefaultMakeSpan::default()),
            // Graceful shutdown will wait for outstanding requests to complete. Add a timeout so
            // requests don't hang forever.
            TimeoutLayer::new(Duration::from_secs(10)),
        ));

    let mut listenfd = ListenFd::from_env();
    let listener = match listenfd.take_tcp_listener(0).unwrap() {
        // if we are given a tcp listener on listen fd 0, we use that one
        Some(listener) => {
            listener.set_nonblocking(true).unwrap();
            TcpListener::from_std(listener).unwrap()
        }
        // otherwise fall back to local listening
        None => TcpListener::bind(config.listen_addr).await.unwrap(),
    };

    tracing::debug!("listening on {}", listener.local_addr()?);
    let serve = axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .into_future();

    let watch = watch_job_queue(store.clone(), hydra_client);
    let wake = wake_builders(store.clone());
    let watch_builders = generate_machines_file(store, config.hydra_machines_file);

    tokio::select! {
        r = serve => { r?; },
        r = watch => { r?; },
        r = wake => { r?; },
        r = watch_builders => { r?; },
    };
    Ok(())
}
