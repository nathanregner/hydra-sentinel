use tokio::sync::{oneshot, watch};

use crate::websocket::ConnectionState;

#[cfg(feature = "tray-icon")]
mod app;
mod config;
mod rate_limiter;
mod websocket;

fn main() -> anyhow::Result<()> {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    let (connection_state_tx, connection_state_rx) = watch::channel(ConnectionState::Disconnected);
    let (enabled_tx, enabled_rx) = watch::channel(true);
    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
    rt.spawn(websocket::connect(
        shutdown_tx,
        connection_state_tx,
        enabled_rx,
    ));
    rt.block_on(
        #[cfg(feature = "tray-icon")]
        async move {
            app::run(shutdown_rx, connection_state_rx, enabled_tx)
        },
        #[cfg(not(feature = "tray-icon"))]
        async move {
            drop((connection_state_rx, enabled_tx));
            let _ = shutdown_rx.await;
            Ok(())
        },
    )
}
