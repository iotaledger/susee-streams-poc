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

use std::{
    clone::Clone,
};

use hyper::{
    Body,
    http::{
        Request,
        Response,
        Result,
        StatusCode,
    }
};

use crate::{
    binary_persistence::BinaryPersist,
    http_protocol::{
        ServerDispatch,
        dispatch_request,
    },
    response_cache::{
        WorkerReceiveMessageFromAddress,
        ResponseCache,
        get_cached_response_status,
        log_err_and_respond_500,
        get_http_100_response,
    }
};


use iota_streams::app_channels::api::DefaultF;

use std::thread;
use std::time::Duration;

use crate::client::http::response_cache::CachedResponseStatus;
use iota_streams::app::futures::executor::block_on;
use crate::response_cache::return_cached_response;

#[derive(Clone)]
pub struct HttpClientProxy {
    pub client: Client,
    cache: &'static ResponseCache,
}

impl HttpClientProxy
{
    pub fn new_from_url(url: &str, cache: &'static ResponseCache) -> Self {
        Self {
            client: Client::new_from_url(url),
            cache,
        }
    }

    pub async fn handle_request(&mut self, req: Request<Body>) -> Result<Response<Body>> {
        dispatch_request(req, self).await
    }


}

#[async_trait(?Send)]
impl ServerDispatch for HttpClientProxy {
    async fn send_message<F: 'static + core::marker::Send + core::marker::Sync>(
        self: &mut Self, message: &TangleMessage<F>) -> Result<Response<Body>>
    {
        let res = self.client.send_message(message).await;
        match res {
            Ok(_) => Ok(Response::new(Default::default())),
            Err(err) => log_err_and_respond_500(err, "send_message")
        }
    }

    async fn receive_message_from_address(self: &mut Self, address_str: &str) -> Result<Response<Body>> {
        match get_cached_response_status(&self.cache.receive_message_from_address, &address_str) {
            CachedResponseStatus::Uncached => {
                self.cache.worker.add_worker_receive_message_from_address(
                    WorkerReceiveMessageFromAddress{
                        address_str: String::from(address_str),
                });
                get_http_100_response()
            },
            CachedResponseStatus::InProcess => get_http_100_response(),
            CachedResponseStatus::Cached => {
                return_cached_response(&self.cache.receive_message_from_address, address_str)
            }
        }
    }

    async fn receive_messages_from_address(self: &mut Self, address_str: &str) -> Result<Response<Body>> {
        unimplemented!()
    }

    async fn fetch_new_commands(self: &mut Self) -> Result<Response<Body>> {
        unimplemented!()
    }
}