use backon::{ExponentialBuilder, Retryable};

use futures_util::{SinkExt, StreamExt};
use hydra_sentinel::{shutdown_signal, SentinelMessage};
use serde::Deserialize;
use std::time::Duration;

use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use tracing_subscriber::{prelude::*};

use crate::rate_limiter::RateLimiter;

mod rate_limiter;

#[derive(Deserialize)]
struct Config {
    server_addr: String,
    hostname: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = hydra_sentinel::init::<Config>()?;

    let rate_limiter = RateLimiter::new(Duration::from_secs(30));
    let run = async {
        loop {
            rate_limiter.throttle(|| run(&config)).await?;
        }
        #[allow(unreachable_code)]
        anyhow::Ok(())
    };

    let shutdown = shutdown_signal();
    tokio::select! {
        r = run => r,
        _ = shutdown => Ok(()),
    }
}

async fn run(config: &Config) -> anyhow::Result<()> {
    let (mut sender, mut receiver) = (|| async move {
        let (stream, response) = connect_async(format!(
            "ws://{}/ws?hostname={}",
            config.server_addr, config.hostname
        ))
        .await?;
        tracing::info!("Connected to server: {response:?}");
        anyhow::Ok(stream)
    })
    .retry(&ExponentialBuilder::default().with_jitter())
    .notify(|err, dur| tracing::error!(?err, "connect failed, retrying after {dur:?}"))
    .await?
    .split();

    let send_task = async move {
        let mut interval = tokio::time::interval(Duration::from_secs(30));
        loop {
            interval.tick().await;
            sender.send(Message::Ping(vec![])).await?;
        }
        #[allow(unreachable_code)]
        anyhow::Ok(())
    };

    let recv_task = async move {
        let mut awake_handle = None;

        while let Some(msg) = receiver.next().await {
            match msg? {
                Message::Text(msg) => {
                    let keep_awake = match SentinelMessage::try_from(msg.as_str()) {
                        Ok(SentinelMessage::KeepAwake(awake)) => awake,
                        Err(err) => {
                            tracing::warn!(?msg, ?err, "Failed to parse message");
                            continue;
                        }
                    };

                    if keep_awake == awake_handle.is_some() {
                        continue;
                    }

                    if keep_awake {
                        tracing::info!("Server requested keep-awake");
                        awake_handle = Some(
                            keepawake::Builder::default()
                                .display(false)
                                .idle(true)
                                .sleep(true)
                                .reason("Build queued")
                                .app_name("Nix Hydra Builder")
                                .app_reverse_domain("net.nregner.hydra-util")
                                .create()?,
                        );
                    } else {
                        tracing::info!("Server cancelled keep-awake");
                        awake_handle = None;
                    }
                }
                Message::Close(_) => {
                    tracing::info!("Server closed connection");
                    break;
                }
                Message::Ping(_) => {}
                Message::Pong(_) => {}
                Message::Frame(_) => {}
                Message::Binary(_) => {}
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
