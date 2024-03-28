use crate::hydra::client::HydraClient;
use axum::{routing::get, Router};
use figment::{providers::Env, Figment};
use listenfd::ListenFd;
use secrecy::SecretString;
use serde::Deserialize;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tower_http::trace::{DefaultMakeSpan, TraceLayer};
use tracing::level_filters::LevelFilter;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

mod builder;
mod error;
mod github;
mod hydra;
mod webhook;

#[derive(Deserialize)]
struct Config {
    listen_addr: String,
    github_webhook_secret: SecretString,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .from_env()
                .unwrap(),
        )
        .init();

    let config = Figment::new()
        .merge(Env::prefixed("HYDRA_"))
        .extract::<Config>()?;

    let client = HydraClient::new("https://hydra.nregner.net".parse()?);

    // build our application with some routes
    let app = Router::new()
        // .route("/ws", get(ws_handler))
        // logging so we can see whats going on
        .route(
            "/webhook",
            github::webhook::handler(config.github_webhook_secret),
        )
        .route("/ws", get(hydra::websocket::handler))
        .layer(TraceLayer::new_for_http().make_span_with(DefaultMakeSpan::default()))
        .with_state(client);

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
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;
    Ok(())
}
