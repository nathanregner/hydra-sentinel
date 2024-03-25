//! Based on tokio-tungstenite example websocket client, but with multiple
//! concurrent websocket clients in one package
//!
//! This will connect to a server specified in the SERVER with N_CLIENTS
//! concurrent connections, and then flood some test messages over websocket.
//! This will also print whatever it gets into stdout.
//!
//! Note that this is not currently optimized for performance, especially around
//! stdout mutex management. Rather it's intended to show an example of working with axum's
//! websocket server and how the client-side and server-side code can be quite similar.
//!

mod keep_alive;

use builder_proto::BuilderMessage;
use futures_util::stream::FuturesUnordered;
use futures_util::{SinkExt, StreamExt};
use std::borrow::Cow;
use std::convert::Infallible;
use std::ops::ControlFlow;
use std::time::{Duration, Instant};
use tokio::signal;

// we will use tungstenite for websocket client impl (same library as what axum is using)
use tokio_tungstenite::{
    connect_async,
    tungstenite::protocol::{frame::coding::CloseCode, CloseFrame, Message},
};

const N_CLIENTS: usize = 1; //set to desired number
const SERVER: &str = "ws://127.0.0.1:3000/ws";

use backon::ExponentialBuilder;
use backon::Retryable;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let start_time = Instant::now();

    let spawn = spawn_client("0");
    spawn.retry(&retry).await?;

    let shutdown = shutdown_signal();
    tokio::select! {
        _ = shutdown => {},
        _ = spawn => {},
    };
    Ok(())
}

//creates a client. quietly exits on failure.
async fn spawn_client(hostname: &str) -> anyhow::Result<()> {
    let retry = ExponentialBuilder::default()
        .with_jitter()
        .with_min_delay(Duration::from_secs(1))
        .with_max_delay(Duration::from_secs(60));

    let stream = (|| async move {
        let (stream, response) = connect_async(format!("{SERVER}?hostname={hostname}")).await?;
        println!("Connected to server: {response:?}");
        anyhow::Ok(stream)
    })
    .retry(&retry)
    .notify(|err: &anyhow::Error, dur: Duration| {
        println!("retrying error {:?} with sleeping {:?}", err, dur);
    })
    .await?;

    let (mut sender, mut receiver) = stream.split();

    //spawn an async sender to push some more messages into the server
    let send_task = async move {
        let mut interval = tokio::time::interval(Duration::from_secs(30));
        loop {
            interval.tick().await;
            sender.send(Message::Ping(vec![])).await?;
        }
        #[allow(unreachable_code)]
        anyhow::Ok(())
    };

    //receiver just prints whatever it gets
    let recv_task = async move {
        let _awake = keepawake::Builder::default()
            .display(true)
            .reason("Build queued")
            .app_name("Nix Hydra Builder")
            .app_reverse_domain("net.nregner.hydra-util")
            .create()?;

        while let Some(msg) = receiver.next().await {
            // print message and break if instructed to do so
            match msg? {
                Message::Text(_) | Message::Binary(_) => {
                    let msg = BuilderMessage::try_from(msg)?;

                    continue;
                }
                Message::Close(c) => {
                    println!(">>> somehow got close message without CloseFrame");
                    break;
                }
                _ => {}
            };
        }

        #[allow(unreachable_code)]
        anyhow::Ok(())
    };

    tokio::select! {
        r = send_task => r,
        r = recv_task => r,
    }
}

/// Function to handle messages we get (with a slight twist that Frame variant is visible
/// since we are working with the underlying tungstenite library directly without axum here).
fn process_message(msg: Message) -> anyhow::Result<ControlFlow<(), ()>> {}

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
