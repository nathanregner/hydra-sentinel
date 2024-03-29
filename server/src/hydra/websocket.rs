use axum::extract::connect_info::ConnectInfo;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Query, State};
use axum::response::IntoResponse;
use futures_util::{sink::SinkExt, stream::StreamExt};
use sentinel_protocol::SentinelMessage;
use serde::Deserialize;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};

use super::store::Store;

#[derive(Deserialize)]
pub struct Params {
    hostname: String,
}

/// The handler for the HTTP request (this gets called when the HTTP GET lands at the start
/// of websocket negotiation). After this completes, the actual switching from HTTP to
/// websocket protocol will occur.
/// This is the last point where we can extract TCP/IP metadata such as IP address of the client
/// as well as things from HTTP headers such as user-agent of the browser etc.
pub async fn handler(
    ws: WebSocketUpgrade,
    State(store): State<Arc<Store>>,
    Query(Params {
        hostname: host_name,
    }): Query<Params>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> impl IntoResponse {
    // let user_agent = if let Some(TypedHeader(user_agent)) = user_agent {
    //     user_agent.to_string()
    // } else {
    //     String::from("Unknown browser")
    // };
    // let hostname = "Unknown browser";
    tracing::info!("{host_name:?}@{addr} connected");
    // finalize the upgrade process by returning upgrade callback.
    // we can customize the callback by sending additional info such as address.
    ws.on_upgrade(move |socket| async move {
        let _ = handle_socket(store, host_name, socket, addr).await;
    })
}

/// Actual websocket statemachine (one will be spawned per connection)
#[tracing::instrument(skip_all, fields(%host_name, %who))]
async fn handle_socket(
    store: Arc<Store>,
    host_name: String,
    socket: WebSocket,
    who: SocketAddr,
) -> anyhow::Result<()> {
    let (mut sender, mut receiver) = socket.split();
    sender.send(Message::Ping(vec![])).await?;

    // TODO: throttle
    let store = store.clone();
    let send_task = async move {
        let handle = store.connect(&host_name, Instant::now())?;
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
        while let Some(Ok(msg)) = receiver.next().await {
            tracing::trace!("received message from {who}");
        }
    };

    tokio::select! {
        r = send_task => r,
        _ = recv_task => Ok(()),
    }
}
