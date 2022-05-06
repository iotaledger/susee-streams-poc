use bee_message::{
    prelude::{
        MessageId
    },
};

use core::fmt;
use iota_streams_core::prelude::String;

use bee_rest_api::types::responses::MessageMetadataResponse;

// This is defined originally in iotaledger/iota.rs/src/client.rs
// the definition is copied here (ugly hack to be removed) to avoid a iota-client dependency.
// A cleaner solution could be to provide a "types" feature for iota-client that only exports the
// the types that are needed to persist and binary read tangle packages
#[derive(Debug,  serde::Serialize, Clone, Copy)]
/// Milestone data.
pub struct MilestoneResponse {
    /// Milestone index.
    pub index: u32,
    /// Milestone message id.
    #[serde(rename = "messageId")]
    pub message_id: MessageId,
    /// Milestone timestamp.
    pub timestamp: u64,
}


#[derive(Clone, Debug)]
pub struct Details {
    pub metadata: MessageMetadataResponse,
    pub milestone: Option<MilestoneResponse>,
}

impl fmt::Display for Details {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<metadata={:?}, milestone={:?}>", self.metadata, self.milestone)
    }
}

/// Options for the user Client
#[derive(Clone)]
pub struct SendOptions {
    pub url: String,
    pub local_pow: bool,
}

impl Default for SendOptions {
    fn default() -> Self {
        Self {
            url: "https://chrysalis-nodes.iota.org".to_string(),
            local_pow: true,
        }
    }
}