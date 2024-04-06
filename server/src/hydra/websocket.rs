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
    host_name: String,
}

pub async fn connect(
    ws: WebSocketUpgrade,
    State(store): State<Arc<Store>>,
    Query(Params { host_name }): Query<Params>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> Result<impl IntoResponse, AppError> {
    let Some(handle) = store.connect(&host_name, Instant::now())? else {
        return Err(AppError::from((
            StatusCode::BAD_REQUEST,
            format!("unknown host_name: {host_name}"),
        )));
    };

    tracing::info!("{host_name:?}@{addr} connected");
    Ok(ws.on_upgrade(move |socket| async move {
        match handle_socket(store, &host_name, addr, socket, handle).await {
            Ok(()) => tracing::info!("{host_name:?}@{addr} connected"),
            Err(err) => tracing::error!(?err, "{host_name:?}@{addr} disconnected"),
        }
    }))
}

#[tracing::instrument(skip_all, fields(%host_name, %who))]
async fn handle_socket(
    store: Arc<Store>,
    host_name: &str,
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
            tracing::trace!("received message from {host_name}");
        }
    };

    tokio::select! {
        r = send_task => r,
        _ = recv_task => Ok(()),
    }
}
