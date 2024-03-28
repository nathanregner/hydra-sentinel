pub mod rate_limiter;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub enum BuilderMessage {
    KeepAwake(bool),
}

impl<'m> TryFrom<&'m str> for BuilderMessage {
    type Error = serde_json::Error;

    fn try_from(msg: &'m str) -> Result<Self, Self::Error> {
        serde_json::from_str(&msg)
    }
}
