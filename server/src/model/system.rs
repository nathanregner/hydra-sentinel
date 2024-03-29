use std::fmt::Display;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Hash, Eq, PartialEq, Debug)]
pub enum System {
    #[serde(rename = "x86_64-linux")]
    X86_64Linux,
    #[serde(rename = "i686-linux")]
    I686Linux,
    #[serde(rename = "aarch64-linux")]
    Aarch64Linux,
    #[serde(rename = "x86_64-darwin")]
    X86_64Darwin,
    #[serde(rename = "aarch64-darwin")]
    Aarch64Darwin,
}

impl Display for System {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            System::X86_64Linux => write!(f, "x86_64-linux"),
            System::I686Linux => write!(f, "i686-linux"),
            System::Aarch64Linux => write!(f, "aarch64-linux"),
            System::X86_64Darwin => write!(f, "x86_64-darwin"),
            System::Aarch64Darwin => write!(f, "aarch64-darwin"),
        }
    }
}
