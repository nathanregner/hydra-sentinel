//! Smoke test for wake-from-sleep detection.
//!
//! Run:
//!   cargo run --example wake_test
//!
//! Then induce sleep (macOS: `pmset sleepnow`, Linux: `systemctl suspend`)
//! and wake the machine. Each wake event is logged to stdout.

// Pull in the wake_monitor module directly so this example stays self-contained.
#[path = "../src/wake_monitor.rs"]
mod wake_monitor;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::new("info"))
        .init();

    tracing::info!("wake monitor test started");
    tracing::info!(
        platform = std::env::consts::OS,
        "waiting for sleep/wake events — induce sleep then wake the machine",
    );

    let wake = wake_monitor::spawn();

    let mut count = 0u32;
    loop {
        if count == 0 {
            tracing::info!("waiting for first sleep/wake cycle...");
        } else {
            tracing::info!(count, "waiting for next sleep/wake cycle...");
        }

        wake.notified().await;
        count += 1;

        tracing::info!(count, "woke up from sleep");
    }
}
