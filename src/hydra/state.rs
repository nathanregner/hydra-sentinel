use crate::builder::Builder;
use std::{
    collections::{HashMap, HashSet},
    sync::Mutex,
    time::{Duration, Instant},
};
use tokio::sync::watch::{channel, Receiver, Sender};

pub struct BuilderState {
    builders: HashMap<String, Builder>,
    last_seen: Mutex<HashMap<String, Instant>>,
    queued: Mutex<HashSet<String>>,
    stale_after: Duration,
    changed: Sender<()>,
}

impl BuilderState {
    pub fn new(stale_after: Duration, builders: impl IntoIterator<Item = Builder>) -> Self {
        let (changed, _) = channel(());
        BuilderState {
            builders: builders
                .into_iter()
                .map(|b| (b.host_name.clone(), b))
                .collect(),
            queued: Mutex::new(HashSet::new()),
            last_seen: Mutex::new(HashMap::new()),
            stale_after,
            changed,
        }
    }

    pub fn subscribe(&self) -> Receiver<()> {
        self.changed.subscribe()
    }

    pub fn connect(&self, host_name: &str, instant: Instant) {
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

    pub fn disconnect(&self, host_name: &str) {
        let mut last_seen = self.last_seen.lock().unwrap();

        let changed = last_seen.remove(host_name).is_some();
        drop(last_seen);

        if changed {
            tracing::info!("Deactivated builder: {}", host_name);
            let _ = self.changed.send(());
        }
    }

    pub fn get_connected(&self) -> Vec<&Builder> {
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

    pub fn update_queued(&self, queued: impl IntoIterator<Item = String>) {
        let mut queued = self.queued.lock().unwrap();
        queued.clear();
        queued.extend(queued);
        *queued = queued.into_iter().collect();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subscribe() {
        let state = BuilderState::new(
            Duration::from_secs(60),
            vec![Builder {
                ssh_user: None,
                host_name: "bogus".into(),
                system: "x86_64-linux".into(),
                features: Default::default(),
                mandatory_features: Default::default(),
                max_jobs: None,
                speed_factor: None,
            }],
        );

        let mut sub = state.subscribe();
        assert!(!sub.has_changed().unwrap());

        state.connect("bogus", Instant::now());
        assert!(sub.has_changed().unwrap());
        sub.mark_unchanged();

        state.disconnect("bogus");
        assert!(sub.has_changed().unwrap());
        sub.mark_unchanged();
    }
}
