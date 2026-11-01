use super::{MacAddress, System};
use serde::Deserialize;
use std::collections::{BTreeSet, HashSet};

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct BuildMachineSpec {
    pub host_name: String,

    /// [Nix system types](https://nixos.org/manual/nix/stable/contributing/hacking#system-type)
    pub systems: BTreeSet<System>,
}

/// A machine to keep awake while Hydra has work queued for it. Hydra itself learns about builders
/// when `hydra-builder` connects to the queue runner.
#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct BuildMachine {
    #[serde(flatten)]
    pub spec: BuildMachineSpec,

    /// VMs hosted by this machine, which is woken for their systems too
    #[serde(default)]
    pub vms: Vec<BuildMachineSpec>,

    /// Optional MAC address to trigger wake-on-lan
    pub mac_address: Option<MacAddress>,
}

impl BuildMachine {
    pub fn host_name(&self) -> &str {
        self.spec.host_name.as_str()
    }

    pub fn mac_address(&self) -> Option<MacAddress> {
        self.mac_address
    }

    // TODO: Separate config (this struct) from logic (store)
    pub fn systems(&self) -> HashSet<System> {
        let mut systems = self.spec.systems.iter().copied().collect::<HashSet<_>>();
        for vm in &self.vms {
            systems.extend(vm.systems.iter().copied());
        }
        systems
    }
}
