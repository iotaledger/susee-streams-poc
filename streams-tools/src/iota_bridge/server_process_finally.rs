use hyper::{
    Body,
    http::{
        Response,
        Result,
    }
};

use crate::{
    ok_or_bail_internal_error_response_500,
    http::{
        DispatchScope,
        ServerProcessFinally,
        get_final_http_status,
        http_tools::{
            DispatchedRequestParts,
            get_response_500,
        }
    },
};

use super::{
    LoraWanNodeDataStore,
    dao::LoraWanNode,
    helpers::{
        DispatchScopeKey
    },
};

use iota_streams::core::async_trait;

#[derive(Clone)]
pub struct ProcessFinally {
    lorawan_nodes: LoraWanNodeDataStore,
}

impl ProcessFinally {
    pub fn new(lorawan_nodes: LoraWanNodeDataStore) -> Self {
        Self {
            lorawan_nodes
        }
    }
}

#[async_trait(?Send)]
impl ServerProcessFinally for ProcessFinally {
    async fn process(&self, mut ret_val: Response<Body>, _req_parts: &DispatchedRequestParts, scope: &dyn DispatchScope) -> Result<Response<Body>> {
        if scope.contains_key(DispatchScopeKey::ADD_NEW_LORAWAN_NODE_TO_DB) {
            let add_new_lorawan_node_to_db = ok_or_bail_internal_error_response_500!(scope.get_bool(DispatchScopeKey::ADD_NEW_LORAWAN_NODE_TO_DB));
            if add_new_lorawan_node_to_db {
                let channel_id = ok_or_bail_internal_error_response_500!(scope.get_string(DispatchScopeKey::STREAMS_CHANNEL_ID));
                let dev_eui = ok_or_bail_internal_error_response_500!(scope.get_string(DispatchScopeKey::LORAWAN_DEV_EUI));

                if let Ok(_existing_node_and_serialize_cb) = self.lorawan_nodes.get_item(dev_eui.as_str()) {
                    log::warn!("Attempt to recreate a lorawan_node that already exists.\nDevEUI: '{}'\nStreams-Channel-ID: '{}'\n\
                                    Please use ...compressed.. versions of the streams IOTA-Bridge API functions after initially using uncompressed ones.",
                               dev_eui, channel_id);
                    *ret_val.status_mut() = get_final_http_status(&ret_val.status(), true);
                } else {
                    let new_lorawan_node = LoraWanNode {
                        dev_eui: dev_eui.clone(),
                        streams_channel_id: channel_id.clone()
                    };
                    match self.lorawan_nodes.write_item_to_db(&new_lorawan_node) {
                        Ok(_) => {}
                        Err(err) => return get_response_500(format!("Could not write new lorawan_node to local database: {}", err).as_str())
                    }
                    *ret_val.status_mut() = get_final_http_status(&ret_val.status(), true);
                }
            }
        }
        Ok(ret_val)
    }
}