#![allow(non_snake_case)]

use std::{
    fmt,
    collections::HashSet,
    cell::RefCell,
};

use async_trait::async_trait;

use tokio::time::{
    sleep,
    Duration
};

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
    address::Address,
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
    http::http_tools::{
        RequestBuilderTools,
        get_string_from_response_body,
    },
    streams_transport::streams_transport::{
        STREAMS_TOOLS_CONST_INX_COLLECTOR_PORT,
        STREAMS_TOOLS_CONST_TRANSPORT_PROCESSING_TIME_SECS,
    },
};

use super::dao::message::{
    MessageDataStore,
    MessageDataStoreOptions,
    Message as DaoMessage
};

#[derive(Clone)]
pub struct MessageIndexerOptions {
    pub iota_node: String,
    pub inx_collector_port: u16,
    pub message_data_store: Option<MessageDataStoreOptions>,
    pub throttle_sleep_time_millisecs: Option<u64>,
}

impl MessageIndexerOptions {
    pub fn new(iota_node: String) -> Self {
        let mut ret_val = Self::default();
        ret_val.iota_node = iota_node;
        ret_val
    }

    pub fn get_inx_collector_url(&self) -> String {
        format!("http://{}:{}", self.iota_node, self.inx_collector_port)
    }
}

impl Default for MessageIndexerOptions {
    fn default() -> Self {
        Self {
            iota_node: "127.0.0.1".to_string(),
            inx_collector_port: STREAMS_TOOLS_CONST_INX_COLLECTOR_PORT,
            message_data_store: None,
            throttle_sleep_time_millisecs: None,
        }
    }
}

impl fmt::Display for MessageIndexerOptions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "MessageIndexerOptions:\n     iota_node: {}\n     port: {}",
               self.iota_node, self.inx_collector_port
        )
    }
}

#[derive(Clone)]
pub struct MessageIndexer {
    hyper_client: HyperClient<HttpConnector, Body>,
    options: MessageIndexerOptions,
    local_message_data_store: Option<MessageDataStore>,
    not_existing_messages: RefCell<HashSet<String>>,
}

struct EndpointUris {}

impl EndpointUris {
    pub const GET_BLOCK: &'static str = "/block";

    pub fn get_uri___get_block(tag_hex_str: &str, only_check_existence: bool) -> String {
        let mut ret_val = format!("{}/{}", Self::GET_BLOCK, tag_hex_str);
        if only_check_existence {
            ret_val += "?checkExistence=true";
        }
        ret_val
    }
}

impl MessageIndexer {
    // utf8 encoded bytes for 'susee-'
    const TAG_PREFIX: [u8; 6] = [115, 117, 115, 101, 101, 45];

    pub fn new(options: MessageIndexerOptions) -> MessageIndexer {
        let mut local_message_data_store: Option<MessageDataStore> = None;
        if let Some(msg_data_store_opt) = options.message_data_store.as_ref() {
            local_message_data_store = Some(MessageDataStore::new(msg_data_store_opt.clone()));
        }
        MessageIndexer {
            hyper_client: HyperClient::new(),
            options,
            local_message_data_store,
            not_existing_messages: RefCell::new(HashSet::new()),
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

    fn get_streams_collector_request(&self, msg_index: [u8; 32], only_check_existence: bool) -> LetsResult<(Request<Body>, String, String)> {
        let msg_index_hex_str = hex::encode(msg_index);
        let tag = self.get_tag_value(msg_index).map_err(|e| LetsError::External(
            anyhow!("Error on converting msg_index '{}' into tag. Error: {}", msg_index_hex_str, e)
        ))?;
        let tag_hex_str = hex::encode(tag);
        log::debug!("[fn get_streams_collector_request()] Request for msg_index {}", msg_index_hex_str);
        let url = self.get_url(&EndpointUris::get_uri___get_block(&tag_hex_str, only_check_existence));
        log::debug!("[fn get_streams_collector_request()] posting get request: {}", url);
        let request = RequestBuilderTools::get_request_builder()
            .method("GET")
            .uri(url.clone())
            .body(Body::empty())
            .map_err(|e| LetsError::External(
                anyhow!("Error on building request for msg_index '{}'. Error: {}", msg_index_hex_str, e)
            ))?;
        Ok((request, url, msg_index_hex_str))
    }

    async fn get_messages_by_msg_index_via_inx_collector(&self, msg_index: [u8; 32]) -> LetsResult<Vec<TransportMessage>> {
        let (request, url, msg_index_hex_str) = self.get_streams_collector_request(msg_index, false)?;
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
}

#[async_trait(?Send)]
impl MessageIndex for MessageIndexer {
    async fn get_messages_by_msg_index(&self, msg_index: [u8; 32], address: &Address) -> LetsResult<Vec<TransportMessage>> {
        let mut ret_val = Vec::<TransportMessage>::new();
        let msg_id_hex_str = hex::encode(address.relative());
        if let Some(local_data_store) = self.local_message_data_store.as_ref() {
            if let Ok((message, _)) = local_data_store.get_item(&msg_id_hex_str) {
                if message.wrapped_binary.len() > 0 {
                    let transport_msg = TransportMessage::new(message.wrapped_binary);
                    log::debug!("[fn get_messages_by_msg_index()] Returning message '{}' from local_data_store", msg_id_hex_str);
                    ret_val.push(transport_msg);
                }
            }
        }

        if self.not_existing_messages.borrow().contains(msg_id_hex_str.as_str()) {
            return Ok(Vec::<TransportMessage>::new())
        }

        if ret_val.is_empty() {
            if let Some(throttle_sleep_millisecs) = self.options.throttle_sleep_time_millisecs {
                tokio::time::sleep(tokio::time::Duration::from_millis(throttle_sleep_millisecs)).await;
            }
            match self.get_messages_by_msg_index_via_inx_collector(msg_index).await {
                Ok(messages) => {
                    if let Some(message) = messages.first() {
                        if let Some(local_data_store) = self.local_message_data_store.as_ref() {
                            local_data_store.write_item_to_db(
                                &DaoMessage{
                                    message_id: msg_id_hex_str.clone(),
                                    wrapped_binary: message.body().clone()
                                })
                                .map_err(|e| LetsError::External(e))?;
                        }

                        log::debug!("[fn get_messages_by_msg_index()] Returning message '{}' from inx-collector", msg_id_hex_str);
                        ret_val.push(message.clone())
                    } else {
                        let _has_been_inserted = self.not_existing_messages
                            .borrow_mut()
                            .insert(msg_id_hex_str.clone());
                        return Ok(messages)
                    }
                }
                Err(e) => return Err(e)
            }
        };

        Ok(ret_val)
    }

    fn get_tag_value(&self, msg_index: [u8; 32]) -> LetsResult<Vec<u8>> {
        let mut ret_val = Vec::with_capacity(msg_index.len() + MessageIndexer::TAG_PREFIX.len());
        ret_val.extend_from_slice(&MessageIndexer::TAG_PREFIX);
        ret_val.extend_from_slice(&msg_index);
        Ok(ret_val)
    }

    async fn validate_successful_message_send(&self, msg_index: [u8; 32]) -> LetsResult<bool> {
        sleep(Duration::from_secs_f32(STREAMS_TOOLS_CONST_TRANSPORT_PROCESSING_TIME_SECS)).await;
        let (request, _, msg_index_hex_str) = self.get_streams_collector_request(msg_index, true)?;
        let response = self.hyper_client.request(request)
            .await
            .map_err(|e| LetsError::External(
                anyhow!("Error on validate_successful_message_send for msg_index '{}'. Error: {}", msg_index_hex_str, e)
            ))?;

        if response.status().is_success() {
            let response_str = get_string_from_response_body(response).await.map_err(|e| LetsError::External(
                anyhow!("Error on get_string_from_response_body of streams_collector response for msg_index '{}'. Error: {}", msg_index_hex_str, e)
            ))?;
            Ok(response_str.to_lowercase().trim() == "true")
        } else {
            Ok(false)
        }
    }
}