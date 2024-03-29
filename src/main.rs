use crate::{
    config::Config,
    hydra::{
        client::HydraClient,
        store::{wake_builders, watch_builders, watch_queue, Store},
    },
};
use axum::{routing::get, Router};
use figment::{
    providers::{Env, Format, Toml},
    Figment,
};
use listenfd::ListenFd;
use std::{future::IntoFuture, net::SocketAddr, sync::Arc};
use tokio::net::TcpListener;
use tower_http::trace::{DefaultMakeSpan, TraceLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

mod builder;
mod config;
mod error;
mod github;
mod hydra;
mod webhook;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(
            EnvFilter::builder()
                .with_default_directive("hydra_hooks=DEBUG".parse()?)
                .from_env()
                .unwrap(),
        )
        .init();

    let config = Figment::new()
        .merge(Toml::string(include_str!("./default.toml")))
        .merge(Env::prefixed("HYDRA_"))
        .extract::<Config>()?;

    let hydra_client = HydraClient::new(config.hydra_url);

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
        .route("/ws", get(hydra::websocket::handler))
        .with_state(store.clone())
        .layer(TraceLayer::new_for_http().make_span_with(DefaultMakeSpan::default()));

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
    .into_future();

    let watch = watch_queue(store.clone(), hydra_client);
    let wake = wake_builders(store.clone());
    let watch_builders = watch_builders(store);

    tokio::select! {
        r = serve => { r?; },
        r = watch => { r?; },
        r = wake => { r?; },
        r = watch_builders => { r?; },
    };
    Ok(())
}
