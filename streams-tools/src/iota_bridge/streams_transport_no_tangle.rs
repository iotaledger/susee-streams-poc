#![allow(non_snake_case)]

use std::fmt;

use async_trait::async_trait;

use anyhow::{
    Result as AnyResult,
    anyhow,
};

use hyper::{
    Body,
    Client as HyperClient,
    client::HttpConnector,
    http::{
        Request,
    }
};

use lets::{
    error::{
        Error as LetsError,
        Result
    },
    message::TransportMessage,
    transport::MessageIndex,
};

use streams::{
    Address,
    transport::Transport,
};

use crate::{
    binary_persist::{
        trans_msg_len,
    },
    message_indexer::{
        MessageIndexer,
        MessageIndexerOptions,
    },
    streams_transport::streams_transport::STREAMS_TOOLS_CONST_INX_COLLECTOR_PORT,
    http::http_tools::RequestBuilderTools,
};

struct EndpointUris {}

impl EndpointUris {
    pub const UPLOAD_TAGGED_DATA: &'static str = "/tagged-data";

    pub fn get_uri___upload_tagged_data(tag_hex_str: &str) -> String {
        format!("{}/{}", Self::UPLOAD_TAGGED_DATA, tag_hex_str)
    }
}

#[derive(Clone)]
pub struct StreamsTransportNoTangleOptions {
    pub iota_node: String,
    pub inx_collector_port: u16,
}

impl StreamsTransportNoTangleOptions {
    pub fn new(iota_node: String) -> Self {
        let mut ret_val = Self::default();
        ret_val.iota_node = iota_node;
        ret_val
    }

    pub fn get_inx_collector_url(&self) -> String {
        format!("http://{}:{}", self.iota_node, self.inx_collector_port)
    }
}

impl Default for StreamsTransportNoTangleOptions {
    fn default() -> Self {
        Self {
            iota_node: "127.0.0.1".to_string(),
            inx_collector_port: STREAMS_TOOLS_CONST_INX_COLLECTOR_PORT,
        }
    }
}

impl fmt::Display for StreamsTransportNoTangleOptions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "StreamsTransportNoTangleOptions:\n     iota_node: {}\n     port: {}",
               self.iota_node, self.inx_collector_port
        )
    }
}


pub struct StreamsTransportNoTangle {
    hyper_client: HyperClient<HttpConnector, Body>,
    msg_indexer: MessageIndexer,
    options: StreamsTransportNoTangleOptions,
}

impl StreamsTransportNoTangle {
    pub fn new(options: StreamsTransportNoTangleOptions) -> Self {
        let mut indexer_options= MessageIndexerOptions::new(options.iota_node.clone());
        indexer_options.inx_collector_port = options.inx_collector_port;
        Self {
            hyper_client: HyperClient::new(),
            msg_indexer: MessageIndexer::new(indexer_options),
            options,
        }
    }

    fn get_url(&self, path_and_params: &str) -> String {
        format!("{}{}", self.options.get_inx_collector_url(), path_and_params)
    }

    fn convert_to_body_bytes(&self, tag: &str, msg: TransportMessage) -> AnyResult<Vec<u8>> {
        let payload_hex = hex::encode(msg.as_ref());
        let mut tagged_data_json = format!("{{\
            \"tag\": \"{}\",\
            \"data\": \"{}\"\
        }}", tag, payload_hex);
        tagged_data_json.retain(|c| !c.is_whitespace());
        log::debug!("[fn convert_to_body_bytes()] tagged_data_json is: '{}'", tagged_data_json);
        Ok(tagged_data_json.as_bytes().to_vec())
    }

    fn get_inx_collector_request(&self, tag_hex_str: &str, msg_index_hex_str: &str, body_bytes: Vec<u8>) -> Result<(Request<Body>, String)> {
        let url = self.get_url(&EndpointUris::get_uri___upload_tagged_data(tag_hex_str));
        let request = RequestBuilderTools::get_request_builder()
            .method("POST")
            .uri(url.clone())
            .body(body_bytes.into())
            .map_err(|e| LetsError::External(
                anyhow!("Error on building inx-collector POST request for msg_index '{}'. Error: {}", msg_index_hex_str, e)
            ))?;
        log::debug!("[fn get_inx_collector_request()] Created POST request with url: {}", url);
        Ok((request, url))
    }

    async fn send_message_to_collector(&mut self, address: Address, msg: TransportMessage) -> Result<TransportMessage> {
        let msg_index = address.to_msg_index();
        let msg_index_hex_str = hex::encode(msg_index);
        let tag = self.msg_indexer.get_tag_value(msg_index)?;
        let tag_hex_str = hex::encode(tag);

        let body_bytes = self.convert_to_body_bytes(tag_hex_str.as_str(), msg)
            .map_err(|e| LetsError::External(
                anyhow!("Error on convert_to_body_bytes for msg_index '{}'. Error: {}", msg_index_hex_str, e)
            ))?;

        let (request, _url) = self.get_inx_collector_request(
            tag_hex_str.as_str(),
            msg_index_hex_str.as_str(),
            body_bytes
        )?;

        let response = self.hyper_client.request(request)
            .await
            .map_err(|e| LetsError::External(
                anyhow!("Error on sending POST request for msg_index '{}'. Error: {}", msg_index_hex_str, e)
            ))?;
        if !response.status().is_success() {
            return Err(LetsError::External(
                anyhow!("inx-collector respondet with http error on sending POST request for msg_index '{}'. Status: {}",
                 msg_index_hex_str, response.status())
            ));
        }
        Ok(TransportMessage::new(Vec::<u8>::new()))
    }
}

#[async_trait(?Send)]
impl<'a> Transport<'a> for StreamsTransportNoTangle
{
    type Msg = TransportMessage;
    type SendResponse = TransportMessage;

    /// Send a Streams message using the inx-collector
    async fn send_message(&mut self, address: Address, msg: Self::Msg) -> Result<Self::SendResponse> {
        log::info!("\n[fn send_message()] Sending message {} to collector. Payload size: {}",
                   hex::encode(address.to_msg_index()), trans_msg_len(&msg));
        self.send_message_to_collector(address, msg).await
    }

    /// Receive messages from the inx-collector
    async fn recv_messages(&mut self, address: Address) -> Result<Vec<Self::Msg>> {
        let ret_val = self.msg_indexer.get_messages_by_msg_index(address.to_msg_index(), &address).await;
        match ret_val.as_ref() {
            Ok(msg_vec) => {
                for (idx, msg) in msg_vec.iter().enumerate() {
                    log::info!("[fn recv_messages()] - idx {}: Received message {} from collector. Payload size: {}",
                               idx, hex::encode(address.to_msg_index()), trans_msg_len(&msg))
                }
            },
            _ => ()
        }
        ret_val
    }
}



