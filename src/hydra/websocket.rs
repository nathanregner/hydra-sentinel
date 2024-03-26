use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Query,
    },
    response::IntoResponse,
};
use builder_proto::BuilderMessage;
use serde::Deserialize;

use std::time::Duration;
use std::ops::ControlFlow;
use std::net::SocketAddr;
use axum::extract::connect_info::ConnectInfo;
use futures_util::{sink::SinkExt, stream::StreamExt};

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
    Query(Params { hostname }): Query<Params>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> impl IntoResponse {
    // let user_agent = if let Some(TypedHeader(user_agent)) = user_agent {
    //     user_agent.to_string()
    // } else {
    //     String::from("Unknown browser")
    // };
    // let hostname = "Unknown browser";
    tracing::info!("{hostname:?}@{addr} connected");
    // finalize the upgrade process by returning upgrade callback.
    // we can customize the callback by sending additional info such as address.
    ws.on_upgrade(move |socket| async move { let _ = handle_socket(socket, addr).await; })
}

/// Actual websocket statemachine (one will be spawned per connection)
async fn handle_socket(socket: WebSocket, who: SocketAddr) -> anyhow::Result<()> {
    let (mut sender, mut receiver) = socket.split();
    sender.send(Message::Ping(vec![])).await?;

    let send_task = async move {
        let mut interval = tokio::time::interval(Duration::from_secs(30));
        loop  {
            sender
                .send(Message::Text(serde_json::to_string(&BuilderMessage::KeepAwake(true)).unwrap()))
            .await?;
            interval.tick().await;
        }
        #[allow(unreachable_code)]
        anyhow::Ok(())
    };

    // This second task will receive messages from client and print them on server console
    let recv_task = async move {
        let mut cnt = 0;
        while let Some(Ok(msg)) = receiver.next().await {
            cnt += 1;
            // print message and break if instructed to do so
            if process_message(msg, who).is_break() {
                break;
            }
        }
        cnt
    };

    // If any one of the tasks exit, abort the other.
    tokio::select! {
        r = send_task => r,
        _ = recv_task => Ok(()),
    }
}

/// helper to print contents of messages to stdout. Has special treatment for Close.
fn process_message(msg: Message, who: SocketAddr) -> ControlFlow<(), ()> {
    match msg {
        Message::Text(t) => {
            println!(">>> {who} sent str: {t:?}");
        }
        Message::Binary(d) => {
            println!(">>> {} sent {} bytes: {:?}", who, d.len(), d);
        }
        Message::Close(c) => {
            if let Some(cf) = c {
                println!(
                    ">>> {} sent close with code {} and reason `{}`",
                    who, cf.code, cf.reason
                );
            } else {
                println!(">>> {who} somehow sent close message without CloseFrame");
            }
            return ControlFlow::Break(());
        }

        Message::Pong(v) => {
            println!(">>> {who} sent pong with {v:?}");
        }
        // You should never need to manually handle Message::Ping, as axum's websocket library
        // will do so for you automagically by replying with Pong and copying the v according to
        // spec. But if you need the contents of the pings you can see them here.
        Message::Ping(v) => {
            println!(">>> {who} sent ping with {v:?}");
        }
    }
    ControlFlow::Continue(())
}
