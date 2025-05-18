// source: tokio_util
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time::{Interval, interval};

pub struct RateLimiter {
    interval: Mutex<Interval>,
}

impl RateLimiter {
    pub fn new(period: Duration) -> Self {
        Self {
            interval: Mutex::new(interval(period)),
        }
    }

    pub async fn throttle<Fut, F, T>(&self, f: F) -> T
    where
        Fut: std::future::Future<Output = T>,
        F: FnOnce() -> Fut,
    {
        self.wait().await;
        f().await
    }

    async fn wait(&self) {
        let mut interval = self.interval.lock().await;
        interval.tick().await;
    }
}
