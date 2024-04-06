use super::store::{BuilderHandle, Store};
use crate::error::AppError;
use axum::extract::connect_info::ConnectInfo;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Query, State};
use axum::response::IntoResponse;
use futures_util::{sink::SinkExt, stream::StreamExt};
use hydra_sentinel::SentinelMessage;
use serde::Deserialize;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};

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
    let handle = store.connect(&host_name, Instant::now())?;

    tracing::info!("{host_name:?}@{addr} connected");
    Ok(ws.on_upgrade(move |socket| async move {
        match handle_socket(store, &host_name, addr, socket, Arc::new(handle)).await {
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
    handle: Arc<BuilderHandle>,
) -> Result<(), AppError> {
    let (mut sender, mut receiver) = socket.split();
    sender.send(Message::Ping(vec![])).await?;

    // TODO: throttle
    let send_handle = handle.clone();
    let send_task = async move {
        let mut sub = store.subscribe();
        loop {
            let wanted = send_handle.wanted();
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
        Ok(())
    };

    let recv_handle = handle;
    let recv_task = async move {
        // TODO: update last seen
        while let Some(Ok(_msg)) = receiver.next().await {
            match _msg {
                Message::Text(_) | Message::Binary(_) | Message::Ping(_) | Message::Pong(_) => {
                    tracing::trace!("{host_name} sent heartbeat");
                    recv_handle.heartbeat(Instant::now())?;
                }
                Message::Close(_) => {
                    tracing::trace!("{host_name} closed connection");
                }
            };
        }
        #[allow(unreachable_code)]
        Ok(())
    };

    tokio::select! {
        r = send_task => r,
        r = recv_task => r,
    }
}
