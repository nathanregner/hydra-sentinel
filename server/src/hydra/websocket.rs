use axum::extract::connect_info::ConnectInfo;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Query, State};
use axum::response::IntoResponse;
use futures_util::{sink::SinkExt, stream::StreamExt};
use hydra_sentinel::SentinelMessage;
use reqwest::StatusCode;
use serde::Deserialize;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::error::AppError;

use super::store::{BuilderHandle, Store};

#[derive(Deserialize)]
pub struct Params {
    hostname: String,
}

/// The handler for the HTTP request (this gets called when the HTTP GET lands at the start
/// of websocket negotiation). After this completes, the actual switching from HTTP to
/// websocket protocol will occur.
/// This is the last point where we can extract TCP/IP metadata such as IP address of the client
/// as well as things from HTTP headers such as user-agent of the browser etc.
pub async fn connect(
    ws: WebSocketUpgrade,
    State(store): State<Arc<Store>>,
    Query(Params { hostname }): Query<Params>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> Result<impl IntoResponse, AppError> {
    let Some(handle) = store.connect(&hostname, Instant::now())? else {
        return Err(AppError::from((
            StatusCode::BAD_REQUEST,
            format!("unknown hostname: {hostname}"),
        )));
    };
    // let user_agent = if let Some(TypedHeader(user_agent)) = user_agent {
    //     user_agent.to_string()
    // } else {
    //     String::from("Unknown browser")
    // };
    // let hostname = "Unknown browser";
    tracing::info!("{hostname:?}@{addr} connected");
    // finalize the upgrade process by returning upgrade callback.
    // we can customize the callback by sending additional info such as address.
    Ok(ws.on_upgrade(move |socket| async move {
        match handle_socket(store, &hostname, addr, socket, handle).await {
            Ok(()) => tracing::info!("{hostname:?}@{addr} connected"),
            Err(err) => tracing::error!(?err, "{hostname:?}@{addr} disconnected"),
        }
    }))
}

#[tracing::instrument(skip_all, fields(%hostname, %who))]
async fn handle_socket(
    store: Arc<Store>,
    hostname: &str,
    // TODO: Get rid of these 2 args
    who: SocketAddr,
    socket: WebSocket,
    handle: BuilderHandle,
) -> anyhow::Result<()> {
    let store = store.clone();
    let (mut sender, mut receiver) = socket.split();
    sender.send(Message::Ping(vec![])).await?;

    // TODO: throttle
    let send_task = async move {
        let mut sub = store.subscribe();
        loop {
            let wanted = handle.wanted();
            if wanted {
                tracing::info!("requesting builder stay awake");
            }
            sender
                .send(Message::Text(SentinelMessage::KeepAwake(wanted).into()))
                .await?;

            tokio::select! {
                r = sub.changed() => r?,
                _ = tokio::time::sleep(Duration::from_secs(30)) => {},
            }
        }
        #[allow(unreachable_code)]
        anyhow::Ok(())
    };

    let recv_task = async move {
        // TODO: update last seen
        while let Some(Ok(_msg)) = receiver.next().await {
            tracing::trace!("received message from {hostname}");
        }
    };

    tokio::select! {
        r = send_task => r,
        _ = recv_task => Ok(()),
    }
}
