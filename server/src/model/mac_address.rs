use serde::{de::Visitor, Deserialize, Deserializer};
use std::fmt::{self, Display, Write};

#[derive(Hash, Eq, PartialEq, Clone, Copy, Debug)]
pub struct MacAddress([u8; 6]);

impl AsRef<[u8; 6]> for MacAddress {
    fn as_ref(&self) -> &[u8; 6] {
        &self.0
    }
}

impl Display for MacAddress {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let bytes = &self.0;
        write!(
            f,
            "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5]
        )
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
