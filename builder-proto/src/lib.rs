pub mod rate_limiter;

use serde::{Deserialize, Serialize};
use tungstenite::Message;

#[derive(Serialize, Deserialize)]
pub enum BuilderMessage {
    KeepAwake(bool),
}

impl TryFrom<Message> for BuilderMessage {
    type Error = anyhow::Error;

    fn try_from(msg: Message) -> Result<Self, Self::Error> {
        match msg {
            Message::Text(txt) => Ok(serde_json::from_str(&txt)?),
            Message::Binary(binary) => Ok(serde_json::from_slice(&binary)?),
            _ => Err(anyhow::anyhow!("Invalid message type")),
        }
    }
}

impl TryFrom<BuilderMessage> for Message {
    type Error = anyhow::Error;

    fn try_from(msg: BuilderMessage) -> Result<Self, Self::Error> {
        Ok(serde_json::to_vec(&msg)?.into())
    }
}
