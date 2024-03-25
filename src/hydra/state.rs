use std::{
    collections::{HashMap, HashSet},
    fmt,
    sync::Mutex,
    time::{Duration, Instant},
};

use serde::{Deserialize, Serialize};
use tokio::sync::broadcast::{channel, Receiver, Sender};

pub struct BuilderState {
    last_seen: Mutex<HashMap<String, Instant>>,
    builders: HashMap<String, Builder>,
    stale_after: Duration,
    changed: Sender<()>,
}

impl BuilderState {
    pub fn new(stale_after: Duration, builders: impl IntoIterator<Item = Builder>) -> Self {
        let (changed, _) = channel(2);
        BuilderState {
            last_seen: Mutex::new(HashMap::new()),
            builders: builders
                .into_iter()
                .map(|b| (b.host_name.clone(), b))
                .collect(),
            stale_after,
            changed,
        }
    }

    pub fn subscribe(&self) -> Receiver<()> {
        self.changed.subscribe()
    }

    pub fn activate(&self, host_name: &str, instant: Instant) {
        if !self.builders.contains_key(host_name) {
            tracing::warn!("Ignoring activation of unknown builder: {}", host_name);
            return;
        }

        let mut last_seen = self.last_seen.lock().unwrap();

        let changed = !last_seen.contains_key(host_name);
        last_seen
            .entry(host_name.to_string())
            .and_modify(|prev| *prev = (*prev).max(instant))
            .or_insert(instant);

        drop(last_seen);

        if changed {
            tracing::info!("Activated builder: {}", host_name);
            let _ = self.changed.send(());
        }
    }

    pub fn deactivate(&self, host_name: &str) {
        let mut last_seen = self.last_seen.lock().unwrap();

        let changed = last_seen.remove(host_name).is_some();
        drop(last_seen);

        if changed {
            tracing::info!("Deactivated builder: {}", host_name);
            let _ = self.changed.send(());
        }
    }

    pub fn get_active(&self) -> Vec<&Builder> {
        let mut last_seen = self.last_seen.lock().unwrap();
        let stale = Instant::now() - self.stale_after;

        let mut builders = Vec::new();
        for (host_name, builder) in &self.builders {
            if let Some(at) = last_seen.get(host_name) {
                if *at < stale {
                    builders.push(builder);
                } else {
                    tracing::warn!("Expiring stale builder: {}", host_name);
                    last_seen.remove(host_name);
                }
            }
        }

        builders
    }
}

#[derive(Serialize, Deserialize)]
pub struct Builder {
    ssh_user: Option<String>,
    host_name: String,
    system: String,
    features: HashSet<String>,
    #[serde(default)]
    mandatory_features: HashSet<String>,
    max_jobs: Option<u32>,
    speed_factor: Option<String>,
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
