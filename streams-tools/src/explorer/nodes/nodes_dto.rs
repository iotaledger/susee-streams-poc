use serde::{
    Deserialize,
    Serialize
};

#[derive(Serialize, Deserialize, Debug)]
pub struct Node {
    pub channel_id: String,
}

/*
#[derive(Serialize, Deserialize, Debug)]
pub struct NewNode {
    pub name: String,
    pub msg: Option<String>,
    pub age: Option<i16>,
}
*/

#[derive(Serialize, Deserialize, Debug)]
pub struct NodeConditions {
    pub channel_id_start: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ChannelId {
    pub id: String,
}