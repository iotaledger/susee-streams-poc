use std::str::from_utf8;

use serde::{
    Deserialize,
    Serialize
};

use utoipa::{
    IntoParams,
    ToSchema
};

use streams::Message as StreamsMessage;

#[derive(Serialize, Deserialize, Debug, ToSchema)]
pub struct Message {
    pub id: String,
    pub public_text: String,
    pub private_text_decrypted: String,
}

impl From<StreamsMessage> for Message {
    fn from(streams_msg: StreamsMessage) -> Self {
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
        }
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