use crate::model::BuildMachine;
use ipnet::IpNet;
use serde::Deserialize;
use std::{net::SocketAddr, path::PathBuf, time::Duration};
use url::Url;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    /// Base URL of the Hydra server
    pub hydra_base_url: Url,

    /// Path to the dynamically generated machines spec managed by sentinel
    /// Must be writable
    pub hydra_machines_file: PathBuf,

    /// Address + port to listen on
    pub listen_addr: SocketAddr,

    /// GitHub webhook secret for authenticating push events
    pub github_webhook_secret_file: PathBuf,

    /// Whitelisted builder IPs
    #[serde(default)]
    pub allowed_ips: Vec<IpNet>,

    /// Time after not hearing from a builder that it is considered dead
    #[serde(with = "humantime_serde")]
    pub heartbeat_timeout: Duration,

    /// List of known machine specs
    /// TODO: What about dynamically registered machines?
    #[serde(default)]
    pub build_machines: Vec<BuildMachine>,
}
