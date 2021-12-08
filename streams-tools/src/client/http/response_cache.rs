use std::collections::HashMap;
use std::sync::{RwLock};
use std::time::Duration;
use tokio::task;

use hyper::{
    Body,
    http::{
        Request,
        Response,
        Result,
        StatusCode,
    }
};

use iota_streams::{
    app::{
        transport::{
            Transport,
            tangle::{
                TangleAddress,
                TangleMessage,
                client::{
                    Client,
                }
            },
        },
    },
    core::{
        async_trait,
    },
};
use crate::{HttpClientProxy, BinaryPersist};
use iota_streams::app_channels::api::DefaultF;
use iota_streams::app::futures::executor::block_on;
use std::str::FromStr;
use std::thread;


pub type EndpointCache = RwLock<HashMap<String, Response<Body>>>;

pub trait ResponseCacheWorker {
    fn run(self: &Self, client: &mut Client, cache: &'static ResponseCache) -> Result<()>;
}

pub type WorkerVecReceiveMessageFromAddress = RwLock<Vec<WorkerReceiveMessageFromAddress>>;

pub struct WorkerReceiveMessageFromAddress {
    pub address_str: String
}

impl ResponseCacheWorker for WorkerReceiveMessageFromAddress {
    fn run(self: &Self, client: &mut Client, cache: &'static ResponseCache)  -> Result<()> {
        println!("[WorkerReceiveMessageFromAddress - run] Adding response with 'status 100' into cache to indicate running process");
        let response = get_http_100_response().unwrap();
        cache.receive_message_from_address.write().unwrap().insert(self.address_str.clone(), response).is_none();

        println!("[WorkerReceiveMessageFromAddress - run] fetching message from the tangle via streams API ");
        let address = TangleAddress::from_str(self.address_str.as_str()).unwrap();
        let message = block_on(
            Transport::<TangleAddress, TangleMessage<DefaultF>>::recv_message(client, &address)
        );
        match message {
            Ok(msg) => {
                println!("[WorkerReceiveMessageFromAddress - run] Received TangleMessage");
                let mut buffer: Vec<u8> = Vec::with_capacity(BinaryPersist::needed_size(&msg));
                let size = BinaryPersist::to_bytes(&msg, buffer.as_mut_slice());
                let response = Response::new(buffer.into());

                println!("[WorkerReceiveMessageFromAddress - run] Remove 'status 100' response from cache");
                cache.receive_message_from_address.write().unwrap().remove(self.address_str.as_str()).unwrap();

                println!("[WorkerReceiveMessageFromAddress - run] Insert response into cache");
                cache.receive_message_from_address.write().unwrap().insert(self.address_str.clone(), response).is_none();
                Ok(())
            },
            Err(err) => {
                println!("[WorkerReceiveMessageFromAddress - run] Received error while fetching TangleMessage: {}", err);
                Ok(())
            }
        }
    }
}


pub(crate) fn get_http_100_response() -> Result<Response<Body>> {
    println!("[response_cache.rs - get_http_100_response] Return http 100 CONTINUE to client");
    Response::builder()
        .status(StatusCode::CONTINUE)
        .body(Default::default())
}

pub(crate) fn log_err_and_respond_500(err: anyhow::Error, fn_name: &str) -> Result<Response<Body>> {
    println!("[response_cache.rs - log_err_and_respond_500 called by {}] Error: {}", fn_name, err);
    let builder = Response::builder()
        .status(StatusCode::INTERNAL_SERVER_ERROR);
    builder.body(Default::default())
}

pub struct WorkerForEndpoints {
    pub(crate) receive_message_from_address: WorkerVecReceiveMessageFromAddress,
    stop_worker_loop: RwLock<bool>
}
impl WorkerForEndpoints {
    pub fn new() -> Self {
        Self {
            receive_message_from_address: RwLock::new(Vec::new()),
            stop_worker_loop: RwLock::<bool>::new(false),
        }
    }

    pub fn add_worker_receive_message_from_address(self: &'static Self, worker: WorkerReceiveMessageFromAddress) {
        self.receive_message_from_address.write().unwrap().push(worker);
    }

    pub fn stop_loop(self: &'static Self) {
        *self.stop_worker_loop.write().unwrap() = true;
    }

    pub fn start_loop(self: &'static Self, client: Client, cache: &'static ResponseCache ) {
        task::spawn(async move {
            loop {
                let stop_loop = self.stop_worker_loop.read().unwrap();
                if *stop_loop {
                    break;
                }
                for worker in self.receive_message_from_address.read().unwrap().as_slice() {
                    worker.run(&mut client.clone(), cache);
                }
                thread::sleep(Duration::from_millis(500));
            }
        });
    }
}

pub struct ResponseCache {
    pub receive_message_from_address: EndpointCache,
    pub worker: WorkerForEndpoints,
}

impl ResponseCache {
    pub fn new() -> Self {
        Self {
            receive_message_from_address: RwLock::new(HashMap::new()),
            worker: WorkerForEndpoints::new(),
        }
    }
}

impl Default for ResponseCache {
    fn default() -> Self {
        Self::new()
    }
}

pub enum CachedResponseStatus {
    Uncached,
    InProcess,
    Cached,
}

pub fn get_cached_response_status(endpoint_cache: &EndpointCache, address_str: &str) -> CachedResponseStatus {
    match endpoint_cache.read().unwrap().get(address_str) {
        Some(response) => {
            match response.status() {
                StatusCode::CONTINUE => CachedResponseStatus::InProcess,
                _ => CachedResponseStatus::Cached,
            }
        },
        _ => CachedResponseStatus::Uncached,
    }
}

pub(crate) fn return_cached_response(endpoint_cache: &EndpointCache, address_str: &str) -> Result<Response<Body>> {
    println!("[EndpointCache - return_cached_response] Returning cached response");
    match endpoint_cache.write().unwrap().remove(address_str) {
        Some(response) => Ok(response),
        _ => log_err_and_respond_500(anyhow::Error::msg("Could not fetch response from RequestCache."), "EndpointCache.return_cached_response")
    }
}
