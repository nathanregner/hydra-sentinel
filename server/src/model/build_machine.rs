use super::{MacAddress, System};
use serde::Deserialize;
use std::{
    collections::{BTreeSet, HashSet},
    fmt,
};

/// A (Nix build machine)[https://nixos.org/manual/nix/stable/command-ref/conf-file#conf-builders] specification
#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct BuildMachineSpec {
    pub ssh_user: Option<String>,
    pub host_name: String,

    /// A comma-separated list of [Nix system types](https://nixos.org/manual/nix/stable/contributing/hacking#system-type)
    pub systems: BTreeSet<System>,

    /// A path to the SSH identity file to be used to log in to the remote machine. If omitted, SSH will use its regular identities.
    pub ssh_key: Option<String>,

    /// The maximum number of builds that Nix will execute in parallel on the machine. Typically this should be equal to the number of CPU cores.
    pub max_jobs: Option<u32>,

    /// The "speed factor", indicating the relative speed of the machine as a positive integer. If there are multiple machines of the right type, Nix will prefer the fastest, taking load into account.
    pub speed_factor: Option<u32>,

    /// A comma-separated list of supported [system features](https://nixos.org/manual/nix/stable/command-ref/conf-file#conf-system-features).
    ///
    /// A machine will only be used to build a derivation if all the features in the derivation's `requiredSystemFeatures` attribute are supported by that machine.
    #[serde(default)]
    pub supported_features: BTreeSet<String>,

    /// A comma-separated list of required [system features](https://nixos.org/manual/nix/stable/command-ref/conf-file#conf-system-features).
    ///
    /// A machine will only be used to build a derivation if all the features in the derivation's `requiredSystemFeatures` attribute are supported by that machine.
    #[serde(default)]
    pub mandatory_features: BTreeSet<String>,

    // The (base64-encoded) public host key of the remote machine. If omitted, SSH will use its regular known_hosts file.
    pub public_host_key: Option<String>,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct BuildMachine {
    #[serde(flatten)]
    pub spec: BuildMachineSpec,

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

impl fmt::Display for BuildMachineSpec {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        macro_rules! write_field {
            ($val: expr) => {
                if let Some(val) = $val {
                    write!(f, " {val}")?;
                } else {
                    write!(f, " -")?;
                }
            };
        }

        macro_rules! write_list {
            ($vals: expr) => {
                let mut it = $vals.iter();
                if let Some(val) = it.next() {
                    write!(f, " {}", val)?;
                    for val in it {
                        write!(f, ",{}", val)?;
                    }
                } else {
                    f.write_str(" -")?;
                }
            };
        }

        let BuildMachineSpec {
            ssh_user,
            host_name,
            systems,
            ssh_key,
            max_jobs,
            speed_factor,
            supported_features,
            mandatory_features,
            public_host_key,
        } = &self;

        // hydra does not support ssh-ng; hard-coding
        f.write_str("ssh://")?;
        if let Some(user) = &ssh_user {
            write!(f, "{user}@")?;
        }
        f.write_str(host_name)?;

        write_list!(systems);
        write_list!(ssh_key);
        write_field!(max_jobs);
        write_field!(speed_factor);
        write_list!(supported_features);
        write_list!(mandatory_features);
        write_field!(public_host_key);

        Ok(())
    }
}
