use super::{MacAddress, System};
use serde::Deserialize;
use std::{
    collections::HashSet,
    fmt::{self, Write},
};

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct NixMachine {
    pub ssh_user: Option<String>,
    pub hostname: String,
    pub system: System,
    pub max_jobs: Option<u32>,
    pub speed_factor: Option<u32>,
    #[serde(default)]
    pub supported_features: HashSet<String>,
    #[serde(default)]
    pub mandatory_features: HashSet<String>,
    /// Optional MAC address to trigger wake-on-lan
    pub mac_address: Option<MacAddress>,
}

impl fmt::Display for NixMachine {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let NixMachine {
            ssh_user,
            hostname,
            system,
            max_jobs,
            speed_factor,
            supported_features: features,
            mandatory_features,
            mac_address: _,
        } = &self;

        // hydra does not support ssh-ng
        f.write_str("ssh://")?;
        if let Some(user) = &ssh_user {
            write!(f, "{user}@")?;
        }
        write!(f, "{hostname} {system} ")?;

        if let Some(max_jobs) = max_jobs {
            write!(f, "{max_jobs} ")?;
        } else {
            f.write_str("- ")?;
        }

        if let Some(speed_factor) = speed_factor {
            write!(f, "{speed_factor} ")?;
        } else {
            f.write_str("- ")?;
        }

        let features = features
            .iter()
            .map(|s| s.as_str())
            .collect::<Vec<_>>()
            .join(",");
        f.write_str(if !features.is_empty() { &features } else { "-" })?;
        f.write_char(' ')?;

        let features = mandatory_features
            .iter()
            .map(|s| s.as_str())
            .collect::<Vec<_>>()
            .join(",");
        f.write_str(if !features.is_empty() { &features } else { "-" })?;
        Ok(())
    }
}
