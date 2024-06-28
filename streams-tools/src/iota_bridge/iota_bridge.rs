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
        Transport,
        tangle::Client
    },
    message::TransportMessage
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
    streams_transport_no_tangle::{
        StreamsTransportNoTangle,
        StreamsTransportNoTangleOptions
    }
};

#[derive(Clone)]
pub struct TangleTransportFactory {
    iota_node: String,
}

#[async_trait(?Send)]
impl  TransportFactory for TangleTransportFactory {
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
pub struct NoTangleTransportFactory {
    iota_node: String,
}

#[async_trait(?Send)]
impl  TransportFactory for NoTangleTransportFactory {
    type Output = StreamsTransportNoTangle;

    async fn new_transport<'a>(&self) -> Rc<RefCell<Self::Output>> {
        Rc::new(RefCell::new(
            StreamsTransportNoTangle::new(
                StreamsTransportNoTangleOptions::new(self.iota_node.clone())
            )
        ))
    }
}

#[derive(Clone)]
pub struct IotaBridgeOptions {
    pub iota_node: String,
    pub error_handling: ErrorHandlingStrategy,
    pub use_tangle_transport: bool,
}

impl IotaBridgeOptions {
    pub fn new(iota_node: &str, error_handling: ErrorHandlingStrategy) -> Self {
        Self {
            iota_node: iota_node.to_string(),
            error_handling,
            use_tangle_transport: true,
        }
    }
}

impl fmt::Display for IotaBridgeOptions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "IotaBridgeOptions:\n   iota_node: {}\n   error_handling: {}\n   use_tangle_transport: {}",
               self.iota_node,
               self.error_handling,
               self.use_tangle_transport
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
        let dispatch_streams = if options.use_tangle_transport {
            Self::get_dispatch_streams(
                &options,
                lora_wan_node_store.clone(),
                pending_request_store,
                TangleTransportFactory {iota_node: options.iota_node.clone()}
            )
        } else {
            Self::get_dispatch_streams(
                &options,
                lora_wan_node_store.clone(),
                pending_request_store,
                NoTangleTransportFactory {iota_node: options.iota_node.clone()}
            )
        };

        IotaBridge {
            scope_provide: ServerScopeProvide::new(),
            dispatch_streams,
            dispatch_command: DispatchCommand::new(),
            dispatch_confirm: DispatchConfirm::new(),
            dispatch_lorawan_node: DispatchLoraWanNode::new(lora_wan_node_store.clone()),
            dispatch_lorawan_rest: DispatchLorawanRest::new(),
            process_finally: ProcessFinally::new(lora_wan_node_store, buffered_message_store),
        }
    }

    fn get_dispatch_streams<TransportFactoryT>(
        options: &IotaBridgeOptions,
        lora_wan_node_store: LoraWanNodeDataStore,
        pending_request_store: PendingRequestDataStore,
        trans_factory: TransportFactoryT
    ) -> DispatchStreams
        where
            TransportFactoryT: TransportFactory + 'static,
            for<'b> <TransportFactoryT as TransportFactory>::Output: Transport<'b, Msg = TransportMessage, SendResponse = TransportMessage> + 'static
    {
        let health_checker = HealthChecker::new(HealthCheckerOptions::new(
            options.iota_node.clone(),
            options.use_tangle_transport,
        ));
        DispatchStreams::new(
            options.error_handling.clone(),
            trans_factory,
            lora_wan_node_store,
            pending_request_store,
            health_checker,
        )
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
