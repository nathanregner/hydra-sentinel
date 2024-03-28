use serde::{de::Visitor, Deserialize, Deserializer};
use std::{collections::HashSet, fmt};

#[derive(Hash, Eq, PartialEq, Clone, Copy, Debug)]
pub struct MacAddress([u8; 6]);

impl AsRef<[u8; 6]> for MacAddress {
    fn as_ref(&self) -> &[u8; 6] {
        &self.0
    }
}

impl<'de> Deserialize<'de> for MacAddress {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct V;

        impl<'de> Visitor<'de> for V {
            type Value = MacAddress;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a MAC address in the format 00:11:22:33:44:55")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                let mut bytes = [0u8; 6];
                let mut iter = value.split(':');
                for i in 0..6 {
                    bytes[i] = u8::from_str_radix(
                        iter.next().ok_or_else(|| E::custom("not enough bytes"))?,
                        16,
                    )
                    .map_err(E::custom)?;
                }
                Ok(MacAddress(bytes))
            }
        }

        deserializer.deserialize_str(V)
    }
}

#[derive(Deserialize)]
pub struct Builder {
    pub ssh_user: Option<String>,
    pub host_name: String,
    pub system: String,
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
