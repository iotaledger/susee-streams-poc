use std::{
    clone::Clone,
    rc::Rc,
};

use iota_streams::core::async_trait;

use hyper::{
    Body,
    http::{
        Response,
        Result,
    }
};

use crate::{
    http::{
        ScopeConsume,
        DispatchScope,
        http_protocol_lorawan_node::{
            ServerDispatchLoraWanNode,
            URI_PREFIX_LORAWAN_NODE,
        },
        http_tools::{
            get_response_500,
            get_response_404,
        },
    },
    iota_bridge::{
        LoraWanNodeDataStore,
        dao::LoraWanNode,
    },
};

#[derive(Clone)]
pub struct DispatchLoraWanNode {
    lorawan_nodes: LoraWanNodeDataStore,
    scope: Option<Rc<dyn DispatchScope>>,
}

impl DispatchLoraWanNode
{
    pub fn new(lorawan_nodes: LoraWanNodeDataStore) -> Self {
        Self {
            lorawan_nodes: lorawan_nodes,
            scope: None,
        }
    }
}

#[async_trait(?Send)]
impl ServerDispatchLoraWanNode for DispatchLoraWanNode {

    fn get_uri_prefix(&self) -> &'static str { URI_PREFIX_LORAWAN_NODE }

    async fn create_node(self: &mut Self, dev_eui: &str, channel_id: &str) -> Result<Response<Body>> {
        let new_node = LoraWanNode{
            dev_eui: dev_eui.to_string(),
            streams_channel_id: channel_id.to_string()
        };
        match self.lorawan_nodes.write_item_to_db(&new_node) {
            Ok(primary_key) => {
                if primary_key == new_node.dev_eui {
                    Ok(Response::new(Default::default()))
                } else {
                    get_response_500("Unknown database error")
                }
            }
            Err(err) => return get_response_500(format!("Error: {}", err).as_str())
        }
    }

    async fn get_node(self: &mut Self, dev_eui: &str, only_check_existence: bool) -> Result<Response<Body>> {
        match self.lorawan_nodes.get_item(&dev_eui.to_string()) {
            Ok(node_and_cb) => {
                if only_check_existence {
                    Ok(Response::new(Default::default()))
                } else {
                    match serde_json::to_string(&node_and_cb.0) {
                        Ok(node_json_str) => {
                            Ok(Response::new(node_json_str.into()))
                        }
                        Err(err) => return get_response_500(format!("Could not serialize lorawan_node. Error: {}", err).as_str())
                    }
                }
            }
            Err(_) => return get_response_404("lorawan_node not found")
        }
    }
}

#[async_trait(?Send)]
impl ScopeConsume for DispatchLoraWanNode {
    fn set_scope(&mut self, scope: Rc<dyn DispatchScope>) {
        self.scope = Some(scope);
    }
}