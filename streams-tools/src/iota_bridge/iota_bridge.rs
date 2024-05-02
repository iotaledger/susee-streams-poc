use std::{
    fmt,
    clone::Clone,
    cell::RefCell,
    rc::Rc,
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
    BufferedMessageDataStore,
    ProcessFinally,
    ServerScopeProvide,
    PendingRequestDataStore,
    server_dispatch_streams::TransportFactory,
    error_handling_strategy::ErrorHandlingStrategy,
    streams_node_health::{
        HealthChecker,
        HealthCheckerOptions
    },
};

#[derive(Clone)]
pub struct ClientFactory {
    iota_node: String,
}

#[async_trait(?Send)]
impl  TransportFactory for ClientFactory {
    type Output = Client<MessageIndexer>;

    async fn new_transport<'a>(&self) -> Rc<RefCell<Self::Output>> {
        let indexer = MessageIndexer::new(MessageIndexerOptions::new(self.iota_node.clone()));
        Rc::new(RefCell::new(
            Client::for_node(
                &get_iota_node_url(self.iota_node.as_str()),
                indexer
            ).await.expect("Could not create client for tangle")
        ))
    }
}

#[derive(Clone)]
pub struct IotaBridgeOptions {
    iota_node: String,
    error_handling: ErrorHandlingStrategy,
}

impl IotaBridgeOptions {
    pub fn new(iota_node: &str, error_handling: ErrorHandlingStrategy) -> Self {
        Self {
            iota_node: iota_node.to_string(),
            error_handling
        }
    }
}

impl fmt::Display for IotaBridgeOptions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "IotaBridgeOptions:\n   iota_node: {}\n   error_handling: {}",
               self.iota_node,
               self.error_handling,
        )
    }
}

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
    pub async fn new(options: IotaBridgeOptions, lora_wan_node_store: LoraWanNodeDataStore, pending_request_store: PendingRequestDataStore, buffered_message_store: BufferedMessageDataStore) -> IotaBridge<'a> {
        let client_factory = ClientFactory {iota_node: options.iota_node.clone()};
        let health_checker = HealthChecker::new(HealthCheckerOptions::new(options.iota_node.clone()));
        IotaBridge {
            scope_provide: ServerScopeProvide::new(),
            dispatch_streams: DispatchStreams::new(
                options.error_handling.clone(),
                client_factory.clone(),
                lora_wan_node_store.clone(),
                pending_request_store,
                health_checker,
            ),
            dispatch_command: DispatchCommand::new(),
            dispatch_confirm: DispatchConfirm::new(),
            dispatch_lorawan_node: DispatchLoraWanNode::new(lora_wan_node_store.clone()),//, pending_request_store.clone()),
            dispatch_lorawan_rest: DispatchLorawanRest::new(),
            process_finally: ProcessFinally::new(lora_wan_node_store, buffered_message_store),
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
