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
        Errors,
    },
    app_channels::api::DefaultF,
};


use std::{
    clone::Clone,
    str::FromStr,
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
    binary_persistence::BinaryPersist,
    http_protocol::{
        ServerDispatch,
        MapStreamsErrors,
        dispatch_request,
    }
};

#[derive(Clone)]
pub struct HttpClientProxy {
    client: Client,
}

impl HttpClientProxy
{
    pub fn new_from_url(url: &str) -> Self {
        Self {
            client: Client::new_from_url(url),
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
        println!("[HttpClientProxy - ServerDispatch - send_message] Incoming TangleMessage");
        let res = self.client.send_message(message).await;
        match res {
            Ok(_) => Ok(Response::new(Default::default())),
            Err(err) => HttpClientProxy::log_err_and_respond_500(err, "send_message")
        }
    }

    async fn receive_message_from_address(self: &mut Self, address_str: &str) -> Result<Response<Body>> {
        println!("[HttpClientProxy - ServerDispatch - receive_message_from_address] Incoming request for address: {}", address_str);
        let address = TangleAddress::from_str(address_str).unwrap();
        let message = Transport::<TangleAddress, TangleMessage<DefaultF>>::
            recv_message(&mut self.client, &address).await;
        match message {
            Ok(msg) => {
                let mut buffer: Vec<u8> = vec![0;BinaryPersist::needed_size(&msg)];
                let _size = BinaryPersist::to_bytes(&msg, buffer.as_mut_slice());
                Ok(Response::new(buffer.into()))
            },
            Err(err) => HttpClientProxy::log_err_and_respond_500(err, "receive_message_from_address")
        }
    }

    async fn receive_messages_from_address(self: &mut Self, _address_str: &str) -> Result<Response<Body>> {
        unimplemented!()
    }

    async fn fetch_new_commands(self: &mut Self) -> Result<Response<Body>> {
        unimplemented!()
    }
}

impl HttpClientProxy {
    fn log_err_and_respond_500(err: anyhow::Error, fn_name: &str) -> Result<Response<Body>> {
        println!("[HttpClientProxy - {}] Error: {}", fn_name, err);

        // // Following implementation does not work because currently it is not possible to access
        // // The streams error value. Instead we expect a MessageLinkNotFoundInTangle error to
        // // make the susee POC run at all.
        // // TODO: Check how to access the streams error value and fix the implementation here
        // let streams_error = &MapStreamsErrors::get_indicator_for_uninitialized();
        // for cause in err.chain() {
        //     if let Some(streams_err) = cause.downcast_ref::<Errors>() {
        //         streams_error = streams_err.clone();
        //         break;
        //     }
        // }
        // let mut status_code = MapStreamsErrors::to_http_status_codes(&streams_error);

        let status_code = MapStreamsErrors::to_http_status_codes(&Errors::MessageLinkNotFoundInTangle(String::from("")));
        let builder = Response::builder()
            .status(status_code);
        builder.body(Default::default())
    }
}
