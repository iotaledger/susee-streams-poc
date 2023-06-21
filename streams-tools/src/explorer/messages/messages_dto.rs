use serde::{
    Deserialize,
    Serialize
};

use iota_streams::{
    app_channels::{
        Bytes,
        UnwrappedMessage,
    }
};

#[derive(Serialize, Deserialize, Debug)]
pub struct Message {
    pub id: String,
    pub public_text: String,
    pub private_text_decrypted: String,
}

impl From<UnwrappedMessage> for Message {
    fn from(unw_msg: UnwrappedMessage) -> Self {
        Message {
            id: unw_msg.link.to_string(),
            public_text: unw_msg.body
                .public_payload()
                .and_then(Bytes::as_str)
                .unwrap_or("")
                .to_string(),
            private_text_decrypted: unw_msg.body
                .masked_payload()
                .and_then(Bytes::as_str)
                .unwrap_or("")
                .to_string()
        }
    }
}

pub type MessageList = Vec<Message>;

#[derive(Serialize, Deserialize, Debug)]
pub struct MessageConditions {
    pub channel_id: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MessageId {
    pub id: String,
}