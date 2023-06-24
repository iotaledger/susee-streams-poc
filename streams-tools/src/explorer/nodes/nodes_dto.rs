use serde::{
    Deserialize,
    Serialize
};

use utoipa::{
    IntoParams,
    ToSchema
};

#[derive(Serialize, Deserialize, Debug, ToSchema)]
pub struct Node {
    pub channel_id: String,
}

/// Filter existing nodes
#[derive(Serialize, Deserialize, Debug, IntoParams)]
pub struct NodeConditions {
    /// Optional. Specify the beginning of the Streams channels ID
    #[param(max_length=80, min_length=1, example ="0ec")]
    pub channel_id_start: Option<String>,
}