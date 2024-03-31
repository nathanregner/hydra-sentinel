use crate::model::{MacAddress, NixMachine, System};
use std::{
    collections::{HashMap, HashSet},
    convert::Infallible,
    net::Ipv4Addr,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tokio::{
    fs::File,
    io::AsyncWriteExt,
    net::UdpSocket,
    sync::watch::{channel, error::RecvError, Receiver, Sender},
};

use super::client::HydraClient;

pub struct Store {
    builders: HashMap<String, NixMachine>,
    last_seen: Mutex<HashMap<String, Instant>>,
    queued_systems: Mutex<HashSet<System>>,
    stale_after: Duration,
    changed: Sender<()>,
}

impl Store {
    pub fn new(stale_after: Duration, builders: impl IntoIterator<Item = NixMachine>) -> Self {
        let (changed, _) = channel(());
        Store {
            builders: builders
                .into_iter()
                .map(|b| (b.host_name.clone(), b))
                .collect(),
            last_seen: Mutex::new(HashMap::new()),
            queued_systems: Mutex::new(HashSet::new()),
            stale_after,
            changed,
        }
    }

    pub fn subscribe(&self) -> Receiver<()> {
        self.changed.subscribe()
    }

    pub fn connect<'s>(
        &'s self,
        host_name: &str,
        now: Instant,
    ) -> anyhow::Result<BuilderHandle<'s>> {
        let Some(builder) = self.builders.get(host_name) else {
            anyhow::bail!("Unknown host: {host_name}");
        };

        let mut last_seen = self.last_seen.lock().unwrap();

        // TODO: separate poll
        let changed = !last_seen.contains_key(host_name);
        last_seen
            .entry(host_name.to_string())
            .and_modify(|prev| *prev = (*prev).max(now))
            .or_insert(now);

        drop(last_seen);

        if dbg!(changed) {
            tracing::debug!("builder connected");
            let _ = self.changed.send(());
        } else {
            anyhow::bail!("{host_name} already connected");
        }
        Ok(BuilderHandle {
            store: self,
            builder,
        })
    }

    fn disconnect(&self, host_name: &str) {
        let mut last_seen = self.last_seen.lock().unwrap();

        let changed = last_seen.remove(host_name).is_some();
        drop(last_seen);

        if changed {
            tracing::debug!("disconnected");
            let _ = self.changed.send(());
        }
    }

    pub fn get_connected(&self) -> Vec<&NixMachine> {
        let mut last_seen = self.last_seen.lock().unwrap();

        let mut builders = Vec::new();
        for (host_name, builder) in &self.builders {
            if let Some(at) = last_seen.get(host_name) {
                let elapsed = at.elapsed();
                if elapsed > self.stale_after {
                    tracing::info!("removing stale builder: {host_name}, not seen for {elapsed:?}");
                    last_seen.remove(host_name);
                } else {
                    builders.push(builder);
                }
            }
        }

        builders
    }

    // TODO: cleanup
    pub fn expire_stale(&self) {
        self.get_connected();
    }

    pub fn update_queued(&self, queued: impl IntoIterator<Item = System>) {
        let updated = queued.into_iter().collect();
        let mut current = self.queued_systems.lock().unwrap();
        if *current != updated {
            *current = updated;
            tracing::info!("Queue updated: systems = {:?}", *current);
            let _ = self.changed.send(());
        } else {
            tracing::debug!("Queue unchanged");
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

pub struct BuilderHandle<'s> {
    store: &'s Store,
    builder: &'s NixMachine,
}

impl<'s> BuilderHandle<'s> {
    pub fn wanted(&self) -> bool {
        let current = self.store.queued_systems.lock().unwrap();
        current.contains(&self.builder.system)
    }
}

impl<'s> Drop for BuilderHandle<'s> {
    fn drop(&mut self) {
        self.store.disconnect(&self.builder.host_name)
    }
}

#[tracing::instrument(skip_all)]
pub async fn wake_builders(store: Arc<Store>) -> anyhow::Result<Infallible> {
    let mut sub = store.subscribe();
    loop {
        tokio::select! {
            r = sub.changed() => r?,
            _ = tokio::time::sleep(Duration::from_secs(30)) => {},
        }

        let mac_addresses = store.machines_to_wake();
        if mac_addresses.is_empty() {
            continue;
        }
        if let Err(err) = wake_all(&mac_addresses).await {
            tracing::error!(?err, "Failed to open socket");
        };
    }
}

pub async fn wake_all(mac_addresses: &[MacAddress]) -> anyhow::Result<()> {
    let from_addr = (Ipv4Addr::new(0, 0, 0, 0), 0);
    let socket = UdpSocket::bind(from_addr).await?;
    socket.set_broadcast(true)?;

    // TODO: parallel?
    for mac_address in mac_addresses {
        wake(&socket, *mac_address).await;
    }

    Ok(())
}

#[tracing::instrument(fields(%mac_address))]
pub async fn wake(socket: &UdpSocket, mac_address: MacAddress) {
    let to_addr = (Ipv4Addr::new(255, 255, 255, 255), 9);
    let packet = wake_on_lan::MagicPacket::new(mac_address.as_ref());
    match socket.send_to(packet.magic_bytes(), to_addr).await {
        Ok(_) => tracing::debug!("Sent WOL packet"),
        Err(err) => tracing::error!(?err, "Failed to send WOL packet"),
    }
}

pub async fn watch_job_queue(
    store: Arc<Store>,
    client: HydraClient,
) -> Result<Infallible, RecvError> {
    let mut interval = tokio::time::interval(Duration::from_secs(15));
    loop {
        interval.tick().await;

        let builds = match client.get_queue().await {
            Ok(builds) => builds,
            Err(err) => {
                tracing::warn!(?err, "Failed to poll queue");
                continue;
            }
        };

        // TODO: log
        store.update_queued(builds.into_iter().map(|b| b.system));
    }
}

#[tracing::instrument(skip_all)]
pub async fn generate_machines_file(
    store: Arc<Store>,
    builders_file: PathBuf,
) -> anyhow::Result<Infallible> {
    // TODO: Validate writable before starting...

    let mut current = String::new();
    let mut sub = store.subscribe();
    loop {
        let mut updated = store
            .get_connected()
            .into_iter()
            .map(|b| format!("{}\n", b))
            .collect::<Vec<_>>();
        updated.sort();
        tracing::debug!("{} connected builders", updated.len());
        let updated = updated.join("");

        if current != updated {
            current = updated;
            File::create(&builders_file)
                .await?
                .write_all(current.as_bytes())
                .await?;
            tracing::info!("Regenerated builders file:\n{current}");
        }

        tokio::select! {
            r = sub.changed() => r?,
            _ = tokio::time::sleep(Duration::from_secs(30)) => {},
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subscribe() {
        let store = Store::new(
            Duration::from_secs(60),
            vec![NixMachine {
                ssh_user: None,
                host_name: "bogus".into(),
                system: System::X86_64Linux,
                features: Default::default(),
                mandatory_features: Default::default(),
                max_jobs: None,
                speed_factor: None,
                mac_address: None,
            }],
        );

        let mut sub = store.subscribe();
        assert!(!sub.has_changed().unwrap());

        let handle = store.connect("bogus", Instant::now());
        assert!(sub.has_changed().unwrap());
        sub.mark_unchanged();

        assert!(!sub.has_changed().unwrap());
        drop(handle);
        assert!(sub.has_changed().unwrap());
    }
}
