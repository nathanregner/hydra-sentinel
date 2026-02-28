use crate::{config::Config, rate_limiter::RateLimiter};
use backon::{ExponentialBuilder, Retryable};
use futures_util::{SinkExt, StreamExt};
use hydra_sentinel::{shutdown_signal, SentinelMessage};
use std::time::Duration;
use tokio::sync::{oneshot, watch};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum ConnectionState {
    Connected { keep_awake: bool },
    Disconnected,
}

pub async fn connect(
    shutdown: oneshot::Sender<()>,
    connection_state: watch::Sender<ConnectionState>,
    enabled: watch::Receiver<bool>,
) -> anyhow::Result<()> {
    let config = hydra_sentinel::init::<Config>(&format!("{}=DEBUG", module_path!()))?;

    let reconnect = RateLimiter::new(Duration::from_secs(30));
    let wake = crate::wake_monitor::spawn();
    let connection = async move {
        loop {
            tokio::select! {
                _ = reconnect.wait() => {}
                _ = wake.notified() => {
                    tracing::info!("Wake from sleep detected, reconnecting immediately");
                }
            }
            monitor(&config, &connection_state, &enabled).await?;
        }
    };

    let shutdown_signal = shutdown_signal();
    let res = tokio::select! {
        r = connection => r,
        _ = shutdown_signal => Ok(()),
    };
    drop(shutdown);
    res
}

async fn monitor(
    config: &Config,
    connection_state: &watch::Sender<ConnectionState>,
    enabled: &watch::Receiver<bool>,
) -> anyhow::Result<()> {
    let (mut sender, mut receiver) = (|| async move {
        tracing::info!("Connecting to server: {}...", config.server_addr);
        let (stream, _response) = connect_async(format!(
            "ws://{}/ws?host_name={}",
            config.server_addr, config.host_name
        ))
        .await?;
        let _ = connection_state.send(ConnectionState::Connected { keep_awake: false });
        tracing::info!("Connected");
        anyhow::Ok(stream)
    })
    .retry(
        &ExponentialBuilder::default()
            .with_max_delay(Duration::from_secs(10))
            .without_max_times()
            .with_jitter(),
    )
    .notify(|err, dur| {
        tracing::warn!(?err, "Connect failed, retrying in {dur:?}");
        let _ = connection_state.send(ConnectionState::Disconnected);
    })
    .await?
    .split();

    let send = async move {
        let mut interval = tokio::time::interval(config.heartbeat_interval);
        loop {
            interval.tick().await;
            sender.send(Message::Ping(Default::default())).await?;
        }
        #[allow(unreachable_code)]
        anyhow::Ok(())
    };

    let recv = async move {
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
                    let _ = connection_state.send(ConnectionState::Connected { keep_awake });
                }
                Message::Close(_) => {
                    let _ = connection_state.send(ConnectionState::Disconnected);
                    anyhow::bail!("Server closed connection");
                }
                Message::Ping(_) => {}
                Message::Pong(_) => {}
                Message::Frame(_) => {}
                Message::Binary(_) => {}
            };
        }

        Err(anyhow::anyhow!("End of stream"))
    };

    let mut enabled = enabled.clone();
    let keep_awake = async move {
        let mut awake_handle = None;
        let mut connection_state = connection_state.subscribe();
        loop {
            tokio::select! {
                _ = enabled.changed() => {},
                _ = connection_state.changed() => {},
            };
            let keep_awake = matches!(
                *connection_state.borrow(),
                ConnectionState::Connected { keep_awake: true }
            );
            if keep_awake != awake_handle.is_some() {
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
        }
    };

    tokio::select! {
        r = send => r,
        r = recv => r,
        r = keep_awake => r,
    }
}
