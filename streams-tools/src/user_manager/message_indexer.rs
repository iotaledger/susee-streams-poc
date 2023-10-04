#![allow(non_snake_case)]

use std::fmt;

use async_trait::async_trait;

use anyhow::anyhow;

use hyper::{
    Body,
    body as hyper_body,
    Client as HyperClient,
    client::HttpConnector,
    http::{
        StatusCode,
        Request,
        Response
    }
};

use lets::{
    transport::MessageIndex,
    message::{
        PreparsedMessage,
        TransportMessage
    },
    error::{
        Result as LetsResult,
        Error as LetsError
    },
};

use crate::{
    http::http_tools::RequestBuilderTools,
    streams_transport::streams_transport::STREAMS_TOOLS_CONST_INX_COLLECTOR_PORT,
};

#[derive(Clone)]
pub struct MessageIndexerOptions {
    pub iota_node: String,
    pub port: u16,
}

impl MessageIndexerOptions {
    pub fn new(iota_node: String) -> Self {
        let mut ret_val = Self::default();
        ret_val.iota_node = iota_node;
        ret_val
    }

    pub fn get_inx_collector_url(&self) -> String {
        format!("http://{}:{}", self.iota_node, self.port)
    }
}

impl Default for MessageIndexerOptions {
    fn default() -> Self {
        Self {
            iota_node: "127.0.0.1".to_string(),
            port: STREAMS_TOOLS_CONST_INX_COLLECTOR_PORT
        }
    }
}

impl fmt::Display for MessageIndexerOptions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "MessageIndexerOptions:\n     iota_node: {}\n     port: {}",
               self.iota_node, self.port
        )
    }
}

#[derive(Clone)]
pub struct MessageIndexer {
    hyper_client: HyperClient<HttpConnector, Body>,
    options: MessageIndexerOptions,
}

struct EndpointUris {}

impl EndpointUris {
    pub const GET_BLOCK: &'static str = "/block";

    pub fn get_uri___get_block(tag_hex_str: &str) -> String {
        format!("{}/{}", Self::GET_BLOCK, tag_hex_str)
    }
}

impl MessageIndexer {
    // utf8 encoded bytes for 'susee-'
    const TAG_PREFIX: [u8; 6] = [115, 117, 115, 101, 101, 45];

    pub fn new(options: MessageIndexerOptions) -> MessageIndexer {
        MessageIndexer {
            hyper_client: HyperClient::new(),
            options,
        }
    }

    fn get_url(&self, path_and_params: &str) -> String {
        format!("{}{}", self.options.get_inx_collector_url(), path_and_params)
    }

    async fn get_transport_msg_payload(msg_index_hex_str: &String, response: Response<Body>) -> LetsResult<Vec<TransportMessage>> {
        let message_hex_str: String;
        {
            let block_json_bytes = hyper_body::to_bytes(response.into_body()).await
                .map_err(|e| LetsError::External(anyhow!("Error on reading hyper body for msg_index '{}'. Error: {}", msg_index_hex_str, e)))?;
            let block_json_str = std::str::from_utf8(&block_json_bytes)
                .map_err(|e| LetsError::External(anyhow!("Error on converting body body bytes into block_json_str for msg_index '{}'. Error: {}", msg_index_hex_str, e)))?;
            if let Some(mut data_str_pos) = block_json_str.find("\"data\":") {
                data_str_pos += 10;
                let data_sub_str = &(block_json_str[data_str_pos..]);
                if let Some(data_str_end) = data_sub_str.find("\"") {
                    message_hex_str = data_sub_str[..data_str_end].to_string();
                } else {
                    return Err(LetsError::External(anyhow!("Error on finding payload data end for msg_index '{}'", msg_index_hex_str)))
                }
            } else {
                return Err(LetsError::External(anyhow!("Error on finding payload data for msg_index '{}'", msg_index_hex_str)))
            }
        }

        let payload_data = hex::decode(message_hex_str)
            .map_err(|e| LetsError::External(anyhow!("Error on hex decoding payload_data for msg_index '{}'. Error: {}", msg_index_hex_str, e)))?;

        let transport_msg = TransportMessage::new(payload_data);
        let _preparsed: PreparsedMessage = transport_msg.clone().parse_header().await?;
        Ok(vec![transport_msg])
    }

    fn get_streams_collector_request(&self, msg_index: [u8; 32]) -> LetsResult<(Request<Body>, String, String)> {
        let msg_index_hex_str = hex::encode(msg_index);
        let tag = self.get_tag_value(msg_index).map_err(|e| LetsError::External(
            anyhow!("Error on converting msg_index '{}' into tag. Error: {}", msg_index_hex_str, e)
        ))?;
        let tag_hex_str = hex::encode(tag);
        log::debug!("[MessageIndexer - get_messages_by_msg_index()] Request for msg_index {}", msg_index_hex_str);
        let url = self.get_url(&EndpointUris::get_uri___get_block(&tag_hex_str));
        log::debug!("[MessageIndexer - posting get request: {}", url);
        let request = RequestBuilderTools::get_request_builder()
            .method("GET")
            .uri(url.clone())
            .body(Body::empty())
            .map_err(|e| LetsError::External(
                anyhow!("Error on building request for msg_index '{}'. Error: {}", msg_index_hex_str, e)
            ))?;
        Ok((request, url, msg_index_hex_str))
    }
}

#[async_trait(?Send)]
impl MessageIndex for MessageIndexer {
    async fn get_messages_by_msg_index(&self, msg_index: [u8; 32]) -> LetsResult<Vec<TransportMessage>> {
        let (request, url, msg_index_hex_str) = self.get_streams_collector_request(msg_index)?;
        let response = self.hyper_client.request(request)
            .await
            .map_err(|e| LetsError::External(
                anyhow!("Error on sending request for msg_index '{}'. Error: {}", msg_index_hex_str, e)
            ))?;

        if response.status().is_success() {
            Self::get_transport_msg_payload(&msg_index_hex_str, response).await
        } else {
            match response.status() {
                StatusCode::BAD_REQUEST => {
                    Ok(vec![])
                },
                _ => {
                    Err(LetsError::External(
                        anyhow!("Streams collector responded with unexpected http error for request '{}'.\nmsg_index: '{}'\nStatus: {}",
                            url,
                            msg_index_hex_str,
                            response.status()
                        )
                    ))
                }
            }
        }
    }

    fn get_tag_value(&self, msg_index: [u8; 32]) -> LetsResult<Vec<u8>> {
        let mut ret_val = Vec::with_capacity(msg_index.len() + MessageIndexer::TAG_PREFIX.len());
        ret_val.extend_from_slice(&MessageIndexer::TAG_PREFIX);
        ret_val.extend_from_slice(&msg_index);
        Ok(ret_val)
    }
}