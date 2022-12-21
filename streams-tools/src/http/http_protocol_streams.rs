use hyper::{
    Body,
    http::{
        Request,
        Response,
        Result,
        Method,
        StatusCode,
    }
};

use iota_streams::{
    app::
    {
        transport::tangle::{
            TangleMessage,
            TangleAddress
        },
    },
    app_channels::api::DefaultF,
    core::{
        async_trait,
        Errors,
    },
};

use crate::{
    return_err_bad_request,
    ok_or_bail_http_response,
    binary_persist::{
        BinaryPersist,
        TangleAddressCompressed,
        TangleMessageCompressed,
        binary_persist_iota_bridge_req::{
            IotaBridgeRequestParts,
            HttpMethod,
            HeaderFlags,
        }
    },
    http::{
        ScopeConsume,
        http_tools::{
            DispatchedRequestParts,
            DispatchedRequestStatus,
            get_dev_eui_from_str,
            StreamsToolsHttpResult,
            StreamsToolsHttpError,
            get_response_from_error,
        }
    }
};

use super::{
    http_tools::{
        RequestBuilderTools,
    }
};

use std::fmt::Display;

// TODO s:
// * Create a enum based Uri and parameter management for API endpoints similar to
//   https://github.com/hyperium/http/blob/master/src/method.rs
//
// * Create an errors enum similat to
//   iota-streams-core/src/errors/error_messages.rs

pub struct EndpointUris {}

pub const URI_PREFIX_STREAMS: &'static str = "/message";

impl EndpointUris {
    pub const SEND_MESSAGE: &'static str = "/message/send";
    pub const RECEIVE_MESSAGE_FROM_ADDRESS: &'static str  = "/message";
    pub const SEND_COMPRESSED_MESSAGE: &'static str = "/message/compressed/send";
    pub const RECEIVE_COMPRESSED_MESSAGE_FROM_ADDRESS: &'static str  = "/message/compressed";
}

pub struct QueryParameters {}

impl QueryParameters {
    pub const RECEIVE_MESSAGE_FROM_ADDRESS: &'static str  = "addr";
    pub const RECEIVE_COMPRESSED_MESSAGE_FROM_ADDRESS_MSGID: &'static str  = "msgid";
    pub const RECEIVE_COMPRESSED_MESSAGE_FROM_ADDRESS_DEV_EUI: &'static str  = "deveui";
    pub const SEND_COMPRESSED_MESSAGE_DEV_EUI: &'static str  = "deveui";
}

// Maps Streams Errors to http status codes
//
// TODO:
// Currently only two streams errors are mapped to http status codes which will lead to errors
// in future development. There are possibilities to persist errors in a more generic way e.g.
// https://docs.rs/strum_macros/0.23.1/strum_macros/index.html
//
// Ideally all streams errors can be returned including their values using http headers
// in a generic implementation.

trait MapStreamsErrorsAssociations {
    const NOT_PROVIDED: &'static str;
}

pub struct MapStreamsErrors {}

impl MapStreamsErrorsAssociations for MapStreamsErrors{
    const NOT_PROVIDED: &'static str = "Not provided";
}

impl MapStreamsErrors {
    pub fn to_http_status_codes(streams_error: &Errors) -> StatusCode {
        match streams_error {
            Errors::MessageLinkNotFoundInTangle(_) => StatusCode::NOT_EXTENDED,
            Errors::MessageNotUnique(_) => StatusCode::VARIANT_ALSO_NEGOTIATES,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    pub fn from_http_status_codes(http_error: StatusCode, comment: Option<String>) -> Errors {
        let comment = comment.unwrap_or(String::from(MapStreamsErrors::NOT_PROVIDED));
        match http_error {
            StatusCode::NOT_EXTENDED => Errors::MessageLinkNotFoundInTangle(comment),
            StatusCode::VARIANT_ALSO_NEGOTIATES => Errors::MessageNotUnique(comment),
            _ => MapStreamsErrors::get_indicator_for_uninitialized(),
        }
    }

    // Can be used to initialize a Errors variable and make sure that this specific value is not
    // mapped by to_http_status_codes() to a specific http status code
    pub fn get_indicator_for_uninitialized() -> Errors {
        Errors::LengthMismatch(0, 0)
    }
}

#[derive(Clone)]
pub struct RequestBuilderStreams {
    tools: RequestBuilderTools,
}

impl RequestBuilderStreams {
    pub fn new(uri_prefix: &str) -> Self {
        Self {
            tools: RequestBuilderTools::new(uri_prefix)
        }
    }

    pub fn get_send_message_request_parts<MessageT: BinaryPersist>(self: &Self, message: &MessageT, endpoint_uri: &str, is_compressed: bool, dev_eui: Option<String>) -> Result<IotaBridgeRequestParts> {
        let mut uri = self.tools.get_uri(endpoint_uri);
        if let Some(eui) = dev_eui {
            uri = format!("{}?{}={}", uri, QueryParameters::SEND_COMPRESSED_MESSAGE_DEV_EUI, eui)
        }
        let mut buffer: Vec<u8> = vec![0; message.needed_size()];
        message.to_bytes(buffer.as_mut_slice()).expect("Persisting into binary data failed");
        let header_flags = RequestBuilderStreams::get_header_flags(is_compressed, HttpMethod::POST);
        Ok(IotaBridgeRequestParts::new(
            header_flags,
            uri,
            buffer
        ))
    }

    pub fn send_message(self: &Self, message: &TangleMessage) -> Result<Request<Body>> {
        self.get_send_message_request_parts(message, EndpointUris::SEND_MESSAGE, false, None)?
            .into_request(RequestBuilderTools::get_request_builder())
    }

    pub fn send_compressed_message(self: &Self, message: &TangleMessageCompressed, dev_eui: Option<String>) -> Result<Request<Body>> {
        self.get_send_message_request_parts(message, EndpointUris::SEND_COMPRESSED_MESSAGE, true, dev_eui)?
            .into_request(RequestBuilderTools::get_request_builder())
    }

    pub fn get_receive_message_from_address_request_parts<AdressT: Display>(self: &Self, address: &AdressT, endpoint_uri: &str, is_compressed: bool, query_param: &str, dev_eui: Option<String>) -> Result<IotaBridgeRequestParts> {
        let mut uri = format!("{}?{}={}",
                          self.tools.get_uri(endpoint_uri).as_str(),
                          query_param,
                          address.to_string()
        );
        if let Some(eui) = dev_eui {
            uri = format!("{}&{}={}", uri, QueryParameters::SEND_COMPRESSED_MESSAGE_DEV_EUI, eui)
        }
        let header_flags = RequestBuilderStreams::get_header_flags(is_compressed, HttpMethod::GET);
        Ok(IotaBridgeRequestParts::new(
            header_flags,
            uri,
            Vec::<u8>::new()
        ))
    }

    pub fn receive_message_from_address(self: &Self, address: &TangleAddress) -> Result<Request<Body>> {
        self.get_receive_message_from_address_request_parts(
            address,
            EndpointUris::RECEIVE_MESSAGE_FROM_ADDRESS,
            false,
            QueryParameters::RECEIVE_MESSAGE_FROM_ADDRESS,
            None,
        )?
        .into_request(RequestBuilderTools::get_request_builder())
    }

    pub fn receive_compressed_message_from_address(self: &Self, address: &TangleAddressCompressed, dev_eui: Option<String>) -> Result<Request<Body>> {
        self.get_receive_message_from_address_request_parts(
            address,
            EndpointUris::RECEIVE_COMPRESSED_MESSAGE_FROM_ADDRESS,
            true,
            QueryParameters::RECEIVE_COMPRESSED_MESSAGE_FROM_ADDRESS_MSGID,
            dev_eui,
        )?
        .into_request(RequestBuilderTools::get_request_builder())
    }

    fn get_header_flags(is_compressed: bool, method: HttpMethod) -> HeaderFlags {
        let mut header_flags = HeaderFlags::from(method);
        if is_compressed {
            header_flags.insert(HeaderFlags::NEEDS_REGISTERD_LORAWAN_NODE);
        }
        header_flags
    }
}

#[async_trait(?Send)]
pub trait ServerDispatchStreams: ScopeConsume {
    fn get_uri_prefix(&self) -> &'static str;
    async fn send_message<F: 'static + core::marker::Send + core::marker::Sync>(
        self: &mut Self, message: &TangleMessage) -> Result<Response<Body>>;
    async fn receive_message_from_address(self: &mut Self, address_str: &str) -> Result<Response<Body>>;
    async fn receive_messages_from_address(self: &mut Self, address_str: &str) -> Result<Response<Body>>;
    async fn send_compressed_message<F: 'static + core::marker::Send + core::marker::Sync>(
        self: &mut Self, message: &TangleMessageCompressed) -> Result<Response<Body>>;
    async fn receive_compressed_message_from_address(self: &mut Self, msgid: &str, dev_eui_str: &str) -> Result<Response<Body>>;
}

pub async fn dispatch_request_streams(req_parts: &DispatchedRequestParts, callbacks: &mut impl ServerDispatchStreams ) -> Result<Response<Body>> {
    match (&req_parts.method, req_parts.path.as_str()) {

        (&Method::POST, EndpointUris::SEND_MESSAGE) => {
            let tangle_msg: TangleMessage = TangleMessage::try_from_bytes(&req_parts.binary_body).unwrap();
            callbacks.send_message::<DefaultF>(&tangle_msg).await
        },

        (&Method::POST, EndpointUris::SEND_COMPRESSED_MESSAGE) => {
            let mut compressed_tangle_msg: TangleMessageCompressed = TangleMessageCompressed::try_from_bytes(&req_parts.binary_body).unwrap();
            let dev_eui_str = if req_parts.status == DispatchedRequestStatus::DeserializedLorawanRest {
                req_parts.dev_eui.clone()
            } else {
                ok_or_bail_http_response!(get_query_param_send_compressed_message_dev_eui(req_parts))
            };
            compressed_tangle_msg.dev_eui = get_dev_eui_from_str(dev_eui_str.as_str(), "RECEIVE_MESSAGE_FROM_ADDRESS",
                                                                 QueryParameters::SEND_COMPRESSED_MESSAGE_DEV_EUI)?;
            callbacks.send_compressed_message::<DefaultF>(&compressed_tangle_msg).await
        },

        (&Method::GET, EndpointUris::RECEIVE_MESSAGE_FROM_ADDRESS) => {
            let address = ok_or_bail_http_response!(get_query_param_receive_message_from_address(req_parts));
            callbacks.receive_message_from_address(address.as_str()).await
        },

        (&Method::GET, EndpointUris::RECEIVE_COMPRESSED_MESSAGE_FROM_ADDRESS) => {
            let dev_eui_is_optional: bool = req_parts.status == DispatchedRequestStatus::DeserializedLorawanRest;
            let (msgid, dev_eui_from_url) = ok_or_bail_http_response!(
                get_query_params_receive_compressed_message_from_address_msgid_deveui(req_parts, dev_eui_is_optional)
            );
            let dev_eui_str = if req_parts.status == DispatchedRequestStatus::DeserializedLorawanRest {
                req_parts.dev_eui.clone()
            } else {
                dev_eui_from_url
            };
            callbacks.receive_compressed_message_from_address(msgid.as_str(), dev_eui_str.as_str()).await
        },

        // Return the 404 Not Found for other routes.
        _ => req_parts.log_and_return_404("dispatch_request_streams", "")
    }
}

fn get_query_params_receive_compressed_message_from_address_msgid_deveui(req_parts: &DispatchedRequestParts, dev_eui_is_optional: bool) -> StreamsToolsHttpResult<(String, String)>{
    let address_key_val: Vec<_> = req_parts.req_url.query_pairs().collect();
    if (!dev_eui_is_optional && address_key_val.len() != 2) ||
       (dev_eui_is_optional && !(1..2).contains(&(address_key_val.len() as i32))) {
        return_err_bad_request!("[http_protocoll - RECEIVE_COMPRESSED_MESSAGE_FROM_ADDRESS] Wrong number of query parameters.\
                Specify the message_id and device_eui using /{}?{}={}&{}={}",
               EndpointUris::RECEIVE_COMPRESSED_MESSAGE_FROM_ADDRESS,
               QueryParameters::RECEIVE_COMPRESSED_MESSAGE_FROM_ADDRESS_MSGID,
               "<MESSAGE-ID-GOES-HERE>",
               QueryParameters::RECEIVE_COMPRESSED_MESSAGE_FROM_ADDRESS_DEV_EUI,
               "<DEVICE-EUI-GOES-HERE>");
    }
    if address_key_val[0].0 != QueryParameters::RECEIVE_COMPRESSED_MESSAGE_FROM_ADDRESS_MSGID &&
        address_key_val[1].0 != QueryParameters::RECEIVE_COMPRESSED_MESSAGE_FROM_ADDRESS_MSGID {
        return_err_bad_request!("[http_protocoll - RECEIVE_COMPRESSED_MESSAGE_FROM_ADDRESS] Query parameter {} is not specified",
               QueryParameters::RECEIVE_COMPRESSED_MESSAGE_FROM_ADDRESS_MSGID);
    }
    if !dev_eui_is_optional {
        if address_key_val[0].0 != QueryParameters::RECEIVE_COMPRESSED_MESSAGE_FROM_ADDRESS_DEV_EUI &&
            address_key_val[1].0 != QueryParameters::RECEIVE_COMPRESSED_MESSAGE_FROM_ADDRESS_DEV_EUI {
            return_err_bad_request!("[http_protocoll - RECEIVE_COMPRESSED_MESSAGE_FROM_ADDRESS] Query parameter {} is not specified",
               QueryParameters::RECEIVE_COMPRESSED_MESSAGE_FROM_ADDRESS_DEV_EUI);
        }
    }

    let msgid: &str = if address_key_val.len() == 1 {
        // In case of address_key_val.len() == 1 there can only be the msgid value
        &*address_key_val[0].1
    } else {
        if address_key_val[0].0 == QueryParameters::RECEIVE_COMPRESSED_MESSAGE_FROM_ADDRESS_MSGID {
            &*address_key_val[0].1
        } else {
            &*address_key_val[1].1
        }
    };

    let dev_eui: &str = if address_key_val.len() == 1 {
        // In case of address_key_val.len() == 1 there can only be the msgid value
        ""
    } else {
        if address_key_val[0].0 == QueryParameters::RECEIVE_COMPRESSED_MESSAGE_FROM_ADDRESS_DEV_EUI {
            &*address_key_val[0].1
        } else {
            &*address_key_val[1].1
        }
    };

    Ok((String::from(msgid), String::from(dev_eui)))
}

fn get_query_param_send_compressed_message_dev_eui(req_parts: &DispatchedRequestParts) -> StreamsToolsHttpResult<String>{
    let address_key_val: Vec<_> = req_parts.req_url.query_pairs().collect();
    if address_key_val.len() != 1 {
        return_err_bad_request!("[http_protocoll - SEND_COMPRESSED_MESSAGE] Wrong number of query parameters.\
                Specify the device_eui using /{}?{}={}",
               EndpointUris::SEND_COMPRESSED_MESSAGE,
               QueryParameters::SEND_COMPRESSED_MESSAGE_DEV_EUI,
               "<DEVICE-EUI-GOES-HERE>");
    }
    if address_key_val[0].0 != QueryParameters::SEND_COMPRESSED_MESSAGE_DEV_EUI {
        return_err_bad_request!("[http_protocoll - RECEIVE_MESSAGE_FROM_ADDRESS] Query parameter {} is not specified",
               QueryParameters::SEND_COMPRESSED_MESSAGE_DEV_EUI);
    }
    Ok(String::from(&*address_key_val[0].1))
}

fn get_query_param_receive_message_from_address(req_parts: &DispatchedRequestParts) -> StreamsToolsHttpResult<String>{
    let address_key_val: Vec<_> = req_parts.req_url.query_pairs().collect();
    if address_key_val.len() != 1 {
        return_err_bad_request!("[http_protocoll - RECEIVE_MESSAGE_FROM_ADDRESS] Wrong number of query parameters.\
                Specify the message address using /{}?{}={}",
               EndpointUris::RECEIVE_MESSAGE_FROM_ADDRESS,
               QueryParameters::RECEIVE_MESSAGE_FROM_ADDRESS,
               "<MESSAGE-ADDRESS-GOES-HERE>");
    }
    if address_key_val[0].0 != QueryParameters::RECEIVE_MESSAGE_FROM_ADDRESS {
        return_err_bad_request!("[http_protocoll - RECEIVE_MESSAGE_FROM_ADDRESS] Query parameter {} is not specified",
               QueryParameters::RECEIVE_MESSAGE_FROM_ADDRESS);
    }
    Ok(String::from(&*address_key_val[0].1))
}