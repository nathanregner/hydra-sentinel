use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub enum SentinelMessage {
    KeepAwake(bool),
}

impl<'m> TryFrom<&'m str> for SentinelMessage {
    type Error = serde_json::Error;

    fn try_from(msg: &'m str) -> Result<Self, Self::Error> {
        serde_json::from_str(&msg)
    }
}

impl Into<String> for SentinelMessage {
    fn into(self) -> String {
        serde_json::to_string(&self).expect("to be serializable")
    }
}
