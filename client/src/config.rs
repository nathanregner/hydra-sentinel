use std::time::Duration;

use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub server_addr: String,
    pub host_name: String,
    #[serde(
        with = "humantime_serde",
        default = "Config::default_heartbeat_interval"
    )]
    pub heartbeat_interval: Duration,
}

impl Config {
    fn default_heartbeat_interval() -> Duration {
        Duration::from_secs(30)
    }
}
