use crate::{
    http::http_tools::{
        RequestBuilderTools,
        DispatchedRequestParts,
    },
};

use hyper::{
    Body,
    http::{
        Request,
        Result,
        Method,
    }
};
use iota_streams::core::async_trait;
use crate::http::http_tools::DispatchedRequestStatus;

pub struct EndpointUris {}

pub const URI_PREFIX_LORAWAN_REST: &'static str = "/lorawan-rest";

impl EndpointUris {
    pub const BINARY_REQUEST: &'static str = "/lorawan-rest/binary_request";
}

pub struct QueryParameters {}

impl QueryParameters {
    pub const BINARY_REQUEST: &'static str  = "deveui";
}

#[derive(Clone)]
pub struct RequestBuilderLorawanRest {
    tools: RequestBuilderTools,
}

impl RequestBuilderLorawanRest {
    pub fn new(uri_prefix: &str) -> Self {
        Self {
            tools: RequestBuilderTools::new(uri_prefix)
        }
    }

    pub fn post_binary_request(self: &Self, request_bytes: Vec<u8>, dev_eui: &str) -> Result<Request<Body>> {
        let uri = format!("{}?{}={}",
                          self.tools.get_uri(EndpointUris::BINARY_REQUEST).as_str(),
                          QueryParameters::BINARY_REQUEST,
                          dev_eui
        );
        RequestBuilderTools::get_request_builder()
            .method("POST")
            .uri(uri)
            .body(Body::from(request_bytes.as_slice().to_owned()))
    }
}

#[async_trait(?Send)]
pub trait ServerDispatchLorawanRest {
    fn get_uri_prefix(&self) -> &'static str;
    async fn post_binary_request(self: &mut Self, dev_eui: &str, request_bytes: &[u8] ) -> anyhow::Result<DispatchedRequestParts>;
}

pub async fn dispatch_request_lorawan_rest<'a>(req_parts: &DispatchedRequestParts, callbacks: &'a mut impl ServerDispatchLorawanRest ) -> anyhow::Result<DispatchedRequestParts> {
    match (&req_parts.method, req_parts.path.as_str()) {

        (&Method::POST, EndpointUris::BINARY_REQUEST) => {
            let dev_eui_key_val: Vec<_> = req_parts.req_url.query_pairs().collect();
            if dev_eui_key_val.len() != 1 {
                panic!("[http_protocoll - BINARY_REQUEST] Wrong number of query parameters.\
                Specify the device EUI using /{}?{}={}",
                       EndpointUris::BINARY_REQUEST,
                       QueryParameters::BINARY_REQUEST,
                       "<DEV-EUI-GOES-HERE>")
            }
            if dev_eui_key_val[0].0 != QueryParameters::BINARY_REQUEST {
                panic!("[http_protocoll - BINARY_REQUEST] Query parameter {} is not specified. Instead the specified Query parameter is: {}",
                       QueryParameters::BINARY_REQUEST,
                       dev_eui_key_val[0].0)
            }
            callbacks.post_binary_request(&*dev_eui_key_val[0].1, &req_parts.binary_body).await
        },

        _ => {
            // Return a copy of the original req_parts and set status to LORAWAN_REST_404.
            log::debug!("[dispatch_request_lorawan_rest] could not dispatch method {} for path '{}'. Returning 404.", req_parts.method, req_parts.path);
            Ok(DispatchedRequestParts {
                dev_eui: req_parts.dev_eui.clone(),
                req_url: req_parts.req_url.clone(),
                status: DispatchedRequestStatus::LorawanRest404,
                method: req_parts.method.clone(),
                path: req_parts.path.clone(),
                binary_body:  req_parts.binary_body.clone(),
            })
        }
    }
}