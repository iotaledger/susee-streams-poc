use hyper::{
    Body,
    http::{
        Response,
        Result,
    }
};

use crate::{
    http::{
        DispatchScope,
        ServerProcessFinally,
        get_final_http_status,
        http_tools::{
            DispatchedRequestStatus,
            DispatchedRequestParts,
            get_response_500
        }
    },
    iota_bridge::{
        LoraWanNodeDataStore,
        dao::LoraWanNode,
        helpers::DispatchScopeKeys,
    }
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
    async fn process(&self, mut ret_val: Response<Body>, req_parts: &DispatchedRequestParts, scope: &dyn DispatchScope) -> Result<Response<Body>> {
        if req_parts.status == DispatchedRequestStatus::DeserializedLorawanRest
            && !req_parts.needs_registered_lorawan_node {
            match scope.get_string(DispatchScopeKeys::STREAMS_CHANNEL_ID) {
                Ok(channel_id) => {
                    if let Ok(_existing_node_and_serialize_cb) = self.lorawan_nodes.get_item(req_parts.dev_eui.as_str()) {
                        log::warn!("Attempt to recreate a lorawan_node that already exists.\nDevEUI: '{}'\nStreams-Channel-ID: '{}'\n\
                                    Please use ...compressed.. versions of the streams IOTA-Bridge API functions after initially using uncompressed ones.",
                                   req_parts.dev_eui, channel_id);
                        *ret_val.status_mut() = get_final_http_status(&ret_val.status(), true);
                    } else {
                        let new_lorawan_node = LoraWanNode {
                            dev_eui: req_parts.dev_eui.clone(),
                            streams_channel_id: channel_id.clone()
                        };
                        match self.lorawan_nodes.write_item_to_db(&new_lorawan_node) {
                            Ok(_) => {}
                            Err(err) => return get_response_500(format!("Could not write new lorawan_node to local database: {}", err).as_str())
                        }
                        *ret_val.status_mut() = get_final_http_status(&ret_val.status(), true);
                    }
                },
                Err(err) => return get_response_500(format!("Could not get STREAMS_CHANNEL_ID from DispatchScope: {}", err).as_str())
            }
        }
        Ok(ret_val)
    }
}