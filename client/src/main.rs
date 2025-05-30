use crate::rate_limiter::RateLimiter;
use backon::{ExponentialBuilder, Retryable};
use futures_util::{SinkExt, StreamExt};
use hydra_sentinel::{SentinelMessage, shutdown_signal};
use serde::Deserialize;
use std::time::Duration;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

mod rate_limiter;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Config {
    server_addr: String,
    host_name: String,
    #[serde(
        with = "humantime_serde",
        default = "Config::default_heartbeat_interval"
    )]
    heartbeat_interval: Duration,
}

impl Config {
    fn default_heartbeat_interval() -> Duration {
        Duration::from_secs(30)
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    let config = hydra_sentinel::init::<Config>(&format!("{}=DEBUG", module_path!()))?;

    let reconnect = RateLimiter::new(Duration::from_secs(30));
    let run = async {
        loop {
            reconnect.throttle(|| run(&config)).await?;
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
        tracing::info!("Connecting to server: {}...", config.server_addr);
        let (stream, _response) = connect_async(format!(
            "ws://{}/ws?host_name={}",
            config.server_addr, config.host_name
        ))
        .await?;
        tracing::info!("Connected");
        anyhow::Ok(stream)
    })
    .retry(&ExponentialBuilder::default().with_jitter())
    .notify(|err, dur| tracing::error!(?err, "Connect failed, retrying in {dur:?}"))
    .await?
    .split();

    let send_task = async move {
        let mut interval = tokio::time::interval(config.heartbeat_interval);
        loop {
            interval.tick().await;
            sender.send(Message::Ping(Default::default())).await?;
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
