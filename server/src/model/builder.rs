use super::{MacAddress, System};
use serde::Deserialize;
use std::{
    collections::HashSet,
    fmt::{self, Write},
};

#[derive(Deserialize)]
pub struct Builder {
    pub ssh_user: Option<String>,
    pub host_name: String,
    pub system: System,
    #[serde(default)]
    pub features: HashSet<String>,
    #[serde(default)]
    pub mandatory_features: HashSet<String>,
    pub max_jobs: Option<u32>,
    pub speed_factor: Option<u32>,
    /// Optional MAC address to trigger wake-on-lan
    pub mac_address: Option<MacAddress>,
}

impl fmt::Display for Builder {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let Builder {
            ssh_user,
            host_name,
            system,
            features,
            mandatory_features,
            max_jobs,
            speed_factor,
            mac_address: _,
        } = &self;

        // hydra does not support ssh-ng
        f.write_str("ssh://")?;
        if let Some(user) = &ssh_user {
            write!(f, "{user}@")?;
        }
        write!(f, "{host_name} {system} ")?;

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
        f.write_str(if features.len() > 0 { &features } else { "-" })?;
        f.write_char(' ')?;

        let features = mandatory_features
            .iter()
            .map(|s| s.as_str())
            .collect::<Vec<_>>()
            .join(",");
        f.write_str(if features.len() > 0 { &features } else { "-" })?;
        Ok(())
    }
}
