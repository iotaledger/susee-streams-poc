use std::{
    clone::Clone,
};

use crate::{
    binary_persist::{
        BinaryPersist,
        binary_persist_iota_bridge_req::IotaBridgeRequestParts,
    },
    http::{
        http_protocol_lorawan_rest::{
            ServerDispatchLorawanRest,
            URI_PREFIX_LORAWAN_REST,
        },
        http_tools::{
            DispatchedRequestParts,
            RequestBuilderTools,
            DispatchedRequestStatus,
        }
    },
};
use iota_streams::core::async_trait;

#[derive(Clone)]
pub struct DispatchLorawanRest {}

impl DispatchLorawanRest
{
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait(?Send)]
impl ServerDispatchLorawanRest for DispatchLorawanRest {

    fn get_uri_prefix(&self) -> &'static str { URI_PREFIX_LORAWAN_REST }

    async fn post_binary_request(self: &mut Self, dev_eui: &str, request_bytes: &[u8]) -> anyhow::Result<DispatchedRequestParts> {
        println!("[HttpClientProxy - DispatchLorawanRest] post_binary_request() - Incoming request for dev_eui '{}' with {} bytes length", dev_eui, request_bytes.len());
        let iota_bridge_request_parts = IotaBridgeRequestParts::try_from_bytes(request_bytes)?;
        let needs_registerd_lorawan_node = iota_bridge_request_parts.needs_registerd_lorawan_node();
        println!("[HttpClientProxy - DispatchLorawanRest] post_binary_request() - Request is valid DispatchLorawanRest request\n{}", iota_bridge_request_parts);
        let hyper_request = iota_bridge_request_parts.into_request(RequestBuilderTools::get_request_builder())?;

        let mut ret_val = DispatchedRequestParts::new(hyper_request).await?;
        ret_val.status = DispatchedRequestStatus::DeserializedLorawanRest;
        ret_val.dev_eui = String::from(dev_eui);
        ret_val.needs_registered_lorawan_node = needs_registerd_lorawan_node;
        Ok(ret_val)
    }
}