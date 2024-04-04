use figment::{
    providers::{Env, Format, Json, Toml},
    Figment,
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use tokio::signal;
use tracing_subscriber::{prelude::*, util::SubscriberInitExt, EnvFilter};

#[derive(Serialize, Deserialize)]
pub enum SentinelMessage {
    KeepAwake(bool),
}

impl<'m> TryFrom<&'m str> for SentinelMessage {
    type Error = serde_json::Error;

    fn try_from(msg: &'m str) -> Result<Self, Self::Error> {
        serde_json::from_str(msg)
    }
}

impl From<SentinelMessage> for String {
    fn from(val: SentinelMessage) -> Self {
        serde_json::to_string(&val).expect("to be serializable")
    }
}

pub fn init<C>() -> anyhow::Result<C>
where
    C: DeserializeOwned,
{
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(
            EnvFilter::builder()
                .with_default_directive("hydra_sentinel_server=DEBUG".parse()?)
                .from_env()
                .unwrap(),
        )
        .init();

    let figment = match std::env::args().nth(1) {
        Some(path) => {
            tracing::info!("Loading config from file {}", path);
            let path = std::path::Path::new(&path);
            let ext = path.extension().and_then(|ext| ext.to_str());
            ext.and_then(|ext| {
                Some(match ext {
                    "toml" => Figment::from(Toml::file(path)),
                    "json" => Figment::from(Json::file(path)),
                    _ => return None,
                })
            })
            .ok_or_else(|| anyhow::anyhow!("Unknown config format: {path:?}"))?
        }
        None => Figment::default(),
    };

    Ok(figment
        .merge(Env::prefixed("HYDRA_SENTINEL_"))
        .extract::<C>()?)
}

pub async fn shutdown_signal() {
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
