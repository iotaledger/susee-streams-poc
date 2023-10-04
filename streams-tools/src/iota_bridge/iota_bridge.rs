use std::{
    clone::Clone,
    cell::RefCell,
    rc::Rc,
};

use hyper::{
    Body,
    http::{
        Request,
        Response,
        Result,
    }
};

use lets::{
    transport::tangle::Client,
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
    },
    user_manager::message_indexer::{
        MessageIndexer,
        MessageIndexerOptions,
    },
    helpers::get_iota_node_url,
};

#[derive(Clone)]
pub struct IotaBridge<'a> {
    scope_provide: ServerScopeProvide,
    dispatch_streams: DispatchStreams<Client<MessageIndexer>>,
    dispatch_command: DispatchCommand<'a>,
    dispatch_confirm: DispatchConfirm<'a>,
    dispatch_lorawan_node: DispatchLoraWanNode,
    dispatch_lorawan_rest: DispatchLorawanRest,
    process_finally: ProcessFinally,
}

impl<'a> IotaBridge<'a>
{
    pub async fn new(iota_node: &str, lora_wan_node_store: LoraWanNodeDataStore, pending_request_store: PendingRequestDataStore) -> IotaBridge<'a> {
        let indexer = MessageIndexer::new(MessageIndexerOptions::new(iota_node.to_string()));
        let client = Rc::new(RefCell::new(
            Client::for_node(
                &get_iota_node_url(iota_node),
                indexer
            ).await.expect("Could not create client for tangle")
        ));

        IotaBridge {
            scope_provide: ServerScopeProvide::new(),
            dispatch_streams: DispatchStreams::new(client, lora_wan_node_store.clone(), pending_request_store),
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
