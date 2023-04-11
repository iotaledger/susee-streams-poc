use iota_streams::{
    app::transport::tangle::client::Client,
};

use std::{
    clone::Clone,
};

use hyper::{
    Body,
    http::{
        Request,
        Response,
        Result,
    }
};
use crate::{
    iota_bridge::{
        DispatchStreams,
        DispatchCommand,
        DispatchConfirm,
        DispatchLoraWanNode,
        DispatchLorawanRest,
        LoraWanNodeDataStore,
        ProcessFinally,
        ServerScopeProvide,
        PendingRequestDataStore,
    },
    http::{
        dispatch_request,
        http_server_dispatch::NormalDispatchCallbacks
    }
};

#[derive(Clone)]
pub struct IotaBridge<'a> {
    scope_provide: ServerScopeProvide,
    dispatch_streams: DispatchStreams,
    dispatch_command: DispatchCommand<'a>,
    dispatch_confirm: DispatchConfirm<'a>,
    dispatch_lorawan_node: DispatchLoraWanNode,
    dispatch_lorawan_rest: DispatchLorawanRest,
    process_finally: ProcessFinally,
}

impl<'a> IotaBridge<'a>
{
    pub fn new(url: &str, lora_wan_node_store: LoraWanNodeDataStore, pending_request_store: PendingRequestDataStore) -> Self {
        let client = Client::new_from_url(url);

        Self {
            scope_provide: ServerScopeProvide::new(),
            dispatch_streams: DispatchStreams::new(&client, lora_wan_node_store.clone(), pending_request_store),
            dispatch_command: DispatchCommand::new(),
            dispatch_confirm: DispatchConfirm::new(),
            dispatch_lorawan_node: DispatchLoraWanNode::new(lora_wan_node_store.clone()),//, pending_request_store.clone()),
            dispatch_lorawan_rest: DispatchLorawanRest::new(),
            process_finally: ProcessFinally::new(lora_wan_node_store),

        }
    }

    pub async fn handle_request(&mut self, req: Request<Body>) -> Result<Response<Body>> {
        let mut other_dispatchers = NormalDispatchCallbacks {
            scope_provide: &mut self.scope_provide,
            streams: &mut self.dispatch_streams,
            command: &mut self.dispatch_command,
            confirm: &mut self.dispatch_confirm,
            lorawan_node: &mut self.dispatch_lorawan_node,
            finally: &mut self.process_finally,
        };

        dispatch_request(req, &mut self.dispatch_lorawan_rest, &mut other_dispatchers).await
    }
}
