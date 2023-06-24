use serde::{
    Deserialize,
    Serialize
};

use utoipa::{
    IntoParams,
    ToSchema
};

use iota_streams::{
    app_channels::{
        Bytes,
        UnwrappedMessage,
    }
};

#[derive(Serialize, Deserialize, Debug, ToSchema)]
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

/// Filter existing messages
#[derive(Serialize, Deserialize, Debug, IntoParams)]
pub struct MessageConditions {
    /// Filter by Streams channel-id. Find existing channel-id of existing nodes using the '/node' endpoint
    #[param(required=true, max_length=80, min_length=80, example ="0ec89c9e5e80c25e24e665fadedf58e7948be80d8bf61c270736974ec2cb36090000000000000000")]
    pub channel_id: Option<String>,
}

/// Specify the message id
#[derive(Serialize, Deserialize, Debug, IntoParams)]
pub struct MessageId {
    /// Streams message id (includes the channel id). Message ids can be listed using the '/messages' endpoint.
    #[param(allow_reserved, max_length=105, min_length=105, example ="0ec89c9e5e80c25e24e665fadedf58e7948be80d8bf61c270736974ec2cb36090000000000000000:84d48c0cc279564b467f7e74")]
    pub message_id: String,
}