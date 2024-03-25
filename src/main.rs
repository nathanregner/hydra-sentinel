mod error;
mod github;
mod hydra;
mod webhook;

use std::net::SocketAddr;

use crate::hydra::HydraClient;
use axum::{routing::get, Router};
use listenfd::ListenFd;
use secrecy::SecretString;
use tokio::net::TcpListener;
use tower_http::trace::{DefaultMakeSpan, TraceLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let client = HydraClient::new("https://hydra.nregner.net".parse()?);

    // build our application with some routes
    let app = Router::new()
        // .route("/ws", get(ws_handler))
        // logging so we can see whats going on
        .route(
            "/webhook",
            github::webhook::handler(SecretString::new("secret".to_string())),
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
        None => TcpListener::bind("127.0.0.1:3000").await.unwrap(),
    };

    tracing::debug!("listening on {}", listener.local_addr()?);
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;
    Ok(())
}
