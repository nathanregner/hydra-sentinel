use serde::{Deserialize, Serialize};
use std::{collections::HashSet, fmt};

#[derive(Serialize, Deserialize)]
pub struct Builder {
    pub ssh_user: Option<String>,
    pub host_name: String,
    pub system: String,
    pub features: HashSet<String>,
    #[serde(default)]
    pub mandatory_features: HashSet<String>,
    pub max_jobs: Option<u32>,
    pub speed_factor: Option<String>,
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
        } = &self;

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

        let features = mandatory_features
            .iter()
            .map(|s| s.as_str())
            .collect::<Vec<_>>()
            .join(",");
        f.write_str(if features.len() > 0 { &features } else { "-" })?;
        Ok(())
    }
}
