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
    pub name: String,
    pub external_id: String,
}

/// Filter existing nodes
#[derive(Serialize, Deserialize, Debug, IntoParams)]
pub struct NodeConditions {
    /// Streams channels ID starts with the specified value
    #[param(max_length=80, min_length=1, example ="0ec")]
    pub channel_id_start: Option<String>,
    /// External id equals the specified value
    #[param(min_length=1)]
    pub external_id: Option<String>,
    /// Name starts with the specified value
    #[param(min_length=1)]
    pub name_start: Option<String>,
}

/// Specify the IOTA Streams channel id
#[derive(Serialize, Deserialize, Debug, IntoParams)]
pub struct ChannelId {
    /// Streams channel-id. Channel ids of existing nodes can be listed using the '/node' endpoint
    #[param(max_length=80, min_length=80, example ="0ec89c9e5e80c25e24e665fadedf58e7948be80d8bf61c270736974ec2cb36090000000000000000")]
    pub channel_id: String,
}