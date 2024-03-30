use crate::model::Builder;
use ipnet::IpNet;
use secrecy::SecretString;
use serde::Deserialize;
use std::{path::PathBuf, time::Duration};
use url::Url;

#[derive(Deserialize)]
pub struct Config {
    pub hydra_url: Url,
    pub machines_file: PathBuf,
    pub listen_addr: String,
    pub allowed_ip_ranges: Vec<IpNet>,
    pub github_webhook_secret: SecretString,
    #[serde(with = "humantime_serde")]
    pub builder_timeout: Duration,
    pub builders: Vec<Builder>,
}
