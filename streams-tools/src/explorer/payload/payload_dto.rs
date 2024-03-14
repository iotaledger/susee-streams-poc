use serde::{
    Deserialize,
    Serialize
};

use utoipa::{
    IntoParams,
};

/// Identify the Node that has send the payload
#[derive(Serialize, Deserialize, Debug, IntoParams)]
pub struct DecodeQueryParams {
    /// External ID (in example a LoRaWAN DevEUI). Find  external ids of existing Nodes using the '/node' endpoint
    #[param(max_length=1023, min_length=1, example ="504F53E833055C50")]
    pub external_id: String,
}