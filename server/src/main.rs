use crate::{
    config::Config,
    hydra::{
        client::HydraClient,
        store::{generate_machines_file, wake_builders, watch_job_queue, Store},
    },
    middleware::allowed_ips,
};
use axum::{routing::get, Router};
use figment::{
    providers::{Env, Format, Toml},
    Figment,
};
use figment_file_provider_adapter::FileAdapter;
use listenfd::ListenFd;
use std::{future::IntoFuture, net::SocketAddr, sync::Arc, time::Duration};
use tokio::{net::TcpListener, signal};
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::{DefaultMakeSpan, TraceLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

mod config;
mod error;
mod github;
mod hydra;
mod middleware;
mod model;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(
            EnvFilter::builder()
                .with_default_directive("hydra_sentinel_server=DEBUG".parse()?)
                .from_env()
                .unwrap(),
        )
        .init();

    let config = Figment::new()
        .merge(Toml::string(include_str!("./default.toml")))
        .merge(FileAdapter::wrap(Env::prefixed("HYDRA_")))
        .extract::<Config>()?;

    let hydra_client = HydraClient::new(config.hydra_base_url);

    // build our application with some routes
    let store = Arc::new(Store::new(config.builder_timeout, config.builders));
    let app = Router::new()
        // .route("/ws", get(ws_handler))
        // logging so we can see whats going on
        .route(
            "/webhook",
            github::webhook::handler(config.github_webhook_secret),
        )
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
    let watch_builders = generate_machines_file(store, config.machines_file);

    tokio::select! {
        r = serve => { r?; },
        r = watch => { r?; },
        r = wake => { r?; },
        r = watch_builders => { r?; },
    };
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
