use crate::builder::{Builder, MacAddress};
use std::{
    collections::{HashMap, HashSet},
    net::Ipv4Addr,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tokio::{
    io,
    net::UdpSocket,
    sync::watch::{channel, error::RecvError, Receiver, Sender},
};

use super::client::HydraClient;

pub struct BuilderState {
    builders: HashMap<String, Builder>,
    last_seen: Mutex<HashMap<String, Instant>>,
    queued_systems: Mutex<HashSet<String>>,
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
            queued_systems: Mutex::new(HashSet::new()),
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
            tracing::warn!(?host_name, "Ignoring unknown host");
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
            tracing::debug!(?host_name, "Builder connected");
            let _ = self.changed.send(());
        } else {
            tracing::debug!(?host_name, "Received heartbeat from builder");
        }
    }

    pub fn disconnect(&self, host_name: &str) {
        let mut last_seen = self.last_seen.lock().unwrap();

        let changed = last_seen.remove(host_name).is_some();
        drop(last_seen);

        if changed {
            tracing::debug!(?host_name, "disconnected");
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
                    tracing::debug!(?host_name, "Removed stale builder");
                    last_seen.remove(host_name);
                }
            }
        }

        builders
    }

    // TODO: cleanup
    pub fn expire_stale(&self) {
        self.get_connected();
    }

    pub fn update_queued(&self, queued: impl IntoIterator<Item = String>) {
        let updated = queued.into_iter().collect();
        let mut current = self.queued_systems.lock().unwrap();
        if *current != updated {
            *current = updated;
            tracing::info!("Queue updated: systems = {:?}", *current);
            let _ = self.changed.send(());
        }
    }

    pub fn machines_to_wake(&self) -> Vec<MacAddress> {
        let queued_systems = self.queued_systems.lock().unwrap().clone();
        let connected = self
            .get_connected()
            .iter()
            .map(|b| b.host_name.as_str())
            .collect::<HashSet<_>>();

        self.builders
            .values()
            .filter_map(|builder| {
                let mac_address = builder.mac_address?;
                if !connected.contains(&*builder.host_name)
                    && queued_systems.contains(&builder.system)
                {
                    Some(mac_address)
                } else {
                    None
                }
            })
            .collect()
    }
}

pub enum Never {}

#[tracing::instrument(skip_all)]
pub async fn wake_builders(state: Arc<BuilderState>) -> Result<Never, RecvError> {
    let mut sub = state.subscribe();
    loop {
        tokio::select! {
            r = sub.changed() => r?,
            _ = tokio::time::sleep(Duration::from_secs(30)) => {},
        }

        let mac_addresses = state.machines_to_wake();
        if mac_addresses.is_empty() {
            continue;
        }
        tracing::debug!("Broadcasting WOL packets to {:?}", mac_addresses);
        if let Err(err) = wake_all(&mac_addresses).await {
            tracing::error!(?err, "Failed to broadcast WOL packets");
        };
    }
}

pub async fn wake_all(mac_addresses: &[MacAddress]) -> io::Result<()> {
    let to_addr = (Ipv4Addr::new(255, 255, 255, 255), 9);
    let from_addr = (Ipv4Addr::new(0, 0, 0, 0), 0);
    let socket = UdpSocket::bind(from_addr).await?;
    socket.set_broadcast(true)?;

    // TODO: parallel?
    for mac_address in mac_addresses {
        let packet = wake_on_lan::MagicPacket::new(mac_address.as_ref());
        socket.send_to(packet.magic_bytes(), to_addr).await?;
    }

    Ok(())
}

pub async fn watch_queue(
    state: Arc<BuilderState>,
    client: HydraClient,
) -> Result<Never, RecvError> {
    let mut sub = state.subscribe();
    loop {
        tokio::select! {
            r = sub.changed() => r?,
            _ = tokio::time::sleep(Duration::from_secs(15)) => {},
        }

        let builds = match client.get_queue().await {
            Ok(builds) => builds,
            Err(err) => {
                tracing::warn!(?err, "Failed to poll queue");
                continue;
            }
        };

        // TODO: log
        state.update_queued(builds.into_iter().map(|b| b.system));
    }
}

pub async fn keep_builders_awake(state: Arc<BuilderState>) -> Result<Never, RecvError> {
    let mut sub = state.subscribe();
    loop {
        tokio::select! {
            r = sub.changed() => r?,
            _ = tokio::time::sleep(Duration::from_secs(30)) => {},
        }

        todo!()
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
                mac_address: None,
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
