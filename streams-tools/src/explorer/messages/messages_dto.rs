use std::str::{
    from_utf8,
    FromStr
};

use anyhow::{
    Result,
    anyhow,
};

use serde::{
    Deserialize,
    Serialize
};

use utoipa::{
    IntoParams,
    ToSchema
};

use streams::{
    Message as StreamsMessage,
    Address,
};

#[derive(Serialize, Deserialize, Debug, ToSchema)]
pub struct Message {
    pub id: String,
    pub public_text: String,
    pub private_text_decrypted: String,
    pub msg_index: String,
    pub streams_content: String,
}

impl From<StreamsMessage> for Message {
    fn from(streams_msg: StreamsMessage) -> Self {
        let streams_content = format!("{:?}", streams_msg.content());
        Message {
            id: streams_msg.address.to_string(),
            public_text: from_utf8(streams_msg
                .public_payload()
                .unwrap_or(&[]))
                .unwrap_or("")
                .to_string(),
            private_text_decrypted: from_utf8(streams_msg
                .masked_payload()
                .unwrap_or(&[]))
                .unwrap_or("")
                .to_string(),
            msg_index: hex::encode(streams_msg.address.to_msg_index()),
            streams_content,
        }
    }
}

impl Message {
    pub fn new_from_id(id: String, pub_text: String, priv_text_decrypted: String) -> Result<Self> {
        let address = Address::from_str(id.as_str()).map_err(|e| anyhow!(e))?;
        Ok(Message {
            id,
            public_text: pub_text,
            private_text_decrypted: priv_text_decrypted,
            msg_index: hex::encode(address.to_msg_index()),
            streams_content: "".to_string(),
        })
    }
}

pub type MessageList = Vec<Message>;

/// Filter existing messages
#[derive(Serialize, Deserialize, Debug, IntoParams)]
pub struct MessageConditions {
    /// Filter by Streams channel-id. Find existing channel-id of existing nodes using the '/node' endpoint
    #[param(required=true, max_length=80, min_length=80, example ="cbd12e732e3c6df93c6fc189bf0d0553c2219d644402bae7caa8968aa5ba15dc0000000000000000")]
    pub channel_id: Option<String>,
}

/// Specify the message id
#[derive(Serialize, Deserialize, Debug, IntoParams)]
pub struct MessageId {
    /// Streams message id (includes the channel id). Message ids can be listed using the '/messages' endpoint.
    #[param(allow_reserved, max_length=105, min_length=105, example ="cbd12e732e3c6df93c6fc189bf0d0553c2219d644402bae7caa8968aa5ba15dc0000000000000000:84d48c0cc279564b467f7e74")]
    pub message_id: String,
}