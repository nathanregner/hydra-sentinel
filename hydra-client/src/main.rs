use backon::{ExponentialBuilder, Retryable};
use builder_proto::rate_limiter::RateLimiter;
use builder_proto::BuilderMessage;
use figment::{providers::Env, Figment};
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use std::time::Duration;
use tokio::signal;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use tracing::level_filters::LevelFilter;
use tracing_subscriber::{prelude::*, EnvFilter};
use url::Url;

#[derive(Deserialize)]
struct Config {
    server: Url,
    hostname: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .from_env()
                .unwrap(),
        )
        .init();

    let config = Figment::new()
        .merge(Env::prefixed("HYDRA_"))
        .extract::<Config>()?;

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

//creates a client. quietly exits on failure.
async fn run(config: &Config) -> anyhow::Result<()> {
    let (mut sender, mut receiver) = (|| async move {
        let (stream, response) =
            connect_async(format!("{}?hostname={}", config.server, config.hostname)).await?;
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
                    let keep_awake = match BuilderMessage::try_from(msg.as_str()) {
                        Ok(BuilderMessage::KeepAwake(awake)) => awake,
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
