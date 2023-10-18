use std::{
    clone::Clone,
};

use async_trait::async_trait;

use hyper::{
    Body,
    http::{
        Request,
        Response,
        Result,
    }
};

use lets::{
    transport::{
        tangle::Client
    },
};

use crate::{
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

use super::{
    DispatchStreams,
    DispatchCommand,
    DispatchConfirm,
    DispatchLoraWanNode,
    DispatchLorawanRest,
    LoraWanNodeDataStore,
    ProcessFinally,
    ServerScopeProvide,
    PendingRequestDataStore,
    server_dispatch_streams::TransportFactory
};

#[derive(Clone)]
pub struct ClientFactory {
    iota_node: String,
}

#[async_trait(?Send)]
impl  TransportFactory for ClientFactory {
    type Output = Client<MessageIndexer>;

    async fn new_transport<'a>(&self) -> Box<Self::Output> {
        let indexer = MessageIndexer::new(MessageIndexerOptions::new(self.iota_node.clone()));
        Box::new(
            Client::for_node(
                &get_iota_node_url(self.iota_node.as_str()),
                indexer
            ).await.expect("Could not create client for tangle")
        )
    }
}



#[derive(Clone)]
pub struct IotaBridge<'a> {
    scope_provide: ServerScopeProvide,
    dispatch_streams: DispatchStreams<ClientFactory>,
    dispatch_command: DispatchCommand<'a>,
    dispatch_confirm: DispatchConfirm<'a>,
    dispatch_lorawan_node: DispatchLoraWanNode,
    dispatch_lorawan_rest: DispatchLorawanRest,
    process_finally: ProcessFinally,
}

impl<'a> IotaBridge<'a>
{
    pub async fn new(iota_node: &str, lora_wan_node_store: LoraWanNodeDataStore, pending_request_store: PendingRequestDataStore) -> IotaBridge<'a> {
        let client_factory = ClientFactory {iota_node: iota_node.to_string()};
        IotaBridge {
            scope_provide: ServerScopeProvide::new(),
            dispatch_streams: DispatchStreams::new(client_factory.clone(), lora_wan_node_store.clone(), pending_request_store),
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
