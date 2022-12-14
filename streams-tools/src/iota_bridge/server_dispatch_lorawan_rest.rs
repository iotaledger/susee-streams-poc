use std::{
    clone::Clone,
    rc::Rc,
};

use crate::{
    binary_persist::{
        BinaryPersist,
        binary_persist_iota_bridge_req::IotaBridgeRequestParts,
    },
    http::{
        DispatchScope,
        ScopeConsume,
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

use super::helpers::{
    DispatchScopeValue,
    write_to_scope,
};

use iota_streams::core::async_trait;

#[derive(Clone)]
pub struct DispatchLorawanRest {
    scope: Option<Rc<dyn DispatchScope>>,
}

impl DispatchLorawanRest
{
    pub fn new() -> Self {
        Self {
            scope: None,
        }
    }

    fn write_scope_data(&self, needs_registerd_lorawan_node: bool, dev_eui: &str) {
        if let Some(scope) = &self.scope {
            write_to_scope(scope, DispatchScopeValue::LorawanDevEui(dev_eui.to_string()));
            write_to_scope(scope, DispatchScopeValue::RequestNeedsRegisteredLorawanNode(needs_registerd_lorawan_node));
        }
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
        self.write_scope_data(needs_registerd_lorawan_node, dev_eui);
        Ok(ret_val)
    }
}

#[async_trait(?Send)]
impl ScopeConsume for DispatchLorawanRest {
    fn set_scope(&mut self, scope: Rc<dyn DispatchScope>) {
        self.scope = Some(scope);
    }
}