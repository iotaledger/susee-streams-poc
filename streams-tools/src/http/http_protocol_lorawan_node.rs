#![allow(non_snake_case)]
#![allow(unused_assignments)]

use hyper::{
    Body,
    http::{
        Request,
        Response,
        Result,
        Method,
    }
};

use crate::{
    http::{
        ScopeConsume,
        http_tools::{
            DispatchedRequestParts,
            PathSegments,
            RequestBuilderTools,
            get_response_400,
            get_response_500,
        }
    }
};

use iota_streams::core::async_trait;

// TODO s:
// * Create a enum based Uri and parameter management for API endpoints similar to
//   https://github.com/hyperium/http/blob/master/src/method.rs
//
// * Create an errors enum similat to
//   iota-streams-core/src/errors/error_messages.rs

pub struct EndpointUris {}

pub const URI_PREFIX_LORAWAN_NODE: &'static str = "/lorawan-node";

impl EndpointUris {
    pub const CREATE_NODE: &'static str = "/lorawan-node";
    pub const GET_NODE: &'static str = "/lorawan-node";
    pub const IS_NODE_KNOWN: &'static str = "/lorawan-node";

    pub fn get_uri___create_node(dev_eui: &str) -> String {
        format!("{}/{}", Self::CREATE_NODE, dev_eui)
    }
    pub fn get_uri___get_node(dev_eui: &str) -> String {
        format!("{}/{}", Self::GET_NODE, dev_eui)
    }
}

pub struct QueryParameters {}

impl QueryParameters {
    pub const CREATE_NODE: &'static str  = "channel-id";
    pub const GET_NODE_EXIST: &'static str  = "exist";
}

#[derive(Clone)]
pub struct RequestBuilderLoraWanNode {
    tools: RequestBuilderTools,
}

impl RequestBuilderLoraWanNode {
    pub fn new(uri_prefix: &str) -> Self {
        Self {
            tools: RequestBuilderTools::new(uri_prefix)
        }
    }

    pub fn create_node(self: &Self, dev_eui: &str, channel_id: &str) -> Result<Request<Body>> {
        let uri = format!("{}?{}={}",
                          self.tools.get_uri(EndpointUris::get_uri___create_node(dev_eui).as_str()).as_str(),
                          QueryParameters::CREATE_NODE,
                          channel_id
        );

        RequestBuilderTools::get_request_builder()
            .method("POST")
            .uri(uri)
            // Although this is a POST request the Body is intentionally left empty here as all
            //  needed data to create the lora_wan_node are contained in the url and query params
            .body(Body::empty())
    }

    pub fn get_node(self: &Self, dev_eui: &str) -> Result<Request<Body>> {
        RequestBuilderTools::get_request_builder()
            .method("GET")
            .uri(self.tools.get_uri(EndpointUris::get_uri___get_node(dev_eui).as_str()))
            .body(Body::empty())
    }
}

#[async_trait(?Send)]
pub trait ServerDispatchLoraWanNode: ScopeConsume {
    fn get_uri_prefix(&self) -> &'static str;
    async fn create_node(self: &mut Self, dev_eui: &str, channel_id: &str) -> Result<Response<Body>>;
    async fn get_node(self: &mut Self, dev_eui: &str, only_check_existence: bool) -> Result<Response<Body>>;
}

pub async fn dispatch_request_lorawan_node(req_parts: &DispatchedRequestParts, callbacks: &mut impl ServerDispatchLoraWanNode) -> Result<Response<Body>> {
    let segments = match PathSegments::new_from_path(req_parts.path.as_str()) {
        Ok(seg) => seg,
        Err(err) => return get_response_400(format!(
            "The dev_eui could not be parsed out of the specified url. Error: {}", err).as_str())
    };

    match (&req_parts.method, segments.main.as_str()) {
        (&Method::POST, EndpointUris::CREATE_NODE) => {
            let channel_id_key_val: Vec<_> = req_parts.req_url.query_pairs().collect();
            if channel_id_key_val.len() != 1 {
                panic!("[http_protocoll - CREATE_NODE] Wrong number of query parameters.\
                Specify the dev_eui and channel-id using /{}/{}?{}={}",
                       EndpointUris::CREATE_NODE,
                       "<Device-EUI-GOES-HERE>",
                       QueryParameters::CREATE_NODE,
                       "<Channel-ID-GOES-HERE>")
            }
            if channel_id_key_val[0].0 != QueryParameters::CREATE_NODE {
                panic!("[http_protocoll - CREATE_NODE] Query parameter {} is not specified. Instead the specified Query parameter is: {}",
                       QueryParameters::CREATE_NODE,
                       channel_id_key_val[0].0)
            }
            callbacks.create_node(segments.last.as_str(), channel_id_key_val[0].1.as_ref()).await
        },

        (&Method::GET, EndpointUris::GET_NODE) => {
            let mut only_check_existence: Option<bool> = None;
            let channel_id_key_val: Vec<_> = req_parts.req_url.query_pairs().collect();
            if channel_id_key_val.len() == 0 {
                only_check_existence = Some(false);
            }
            else if channel_id_key_val.len() != 1 {
                panic!("[http_protocoll - GET_NODE] Wrong number of query parameters.\
                Use GET for /{0}/{1} to fetch lorawan node instances or add optional query param {2}\
                to check its existence like this /{0}/{1}?{2}",
                        EndpointUris::CREATE_NODE,
                       "<Device-EUI-GOES-HERE>",
                       QueryParameters::CREATE_NODE)
            }
            else if channel_id_key_val[0].0 != QueryParameters::GET_NODE_EXIST {
                panic!("[http_protocoll - GET_NODE] Query parameter {} is not specified. Instead the specified Query parameter is: {}",
                       QueryParameters::GET_NODE_EXIST,
                       channel_id_key_val[0].0)
            } else {
                only_check_existence = Some(true);
            }

            match only_check_existence {
                Some(check_existence) => {
                    callbacks.get_node(segments.last.as_str(), check_existence).await
                }
                None => return get_response_500("This should have been impossible")
            }
        },

        // Return the 404 Not Found for other routes.
        _ => req_parts.log_and_return_404("dispatch_request_command", "")
    }
}