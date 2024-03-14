use async_trait::async_trait;

use hyper::http::{
    StatusCode,
    Method
};

use lets::address::{
    Address,
    MsgId
};

use crate::{
    explorer::{
        threading_helpers::{
            run_worker_in_own_thread,
            Worker
        },
        error::{
            Result,
            AppError,
        },
        messages::Message,
        app_state::MessagesState,
    },
    user_manager::dao::user::{
        UserDataStore,
    },
    multi_channel_management::{
        get_channel_manager_for_external_id,
        MultiChannelManagerOptions
    },
    binary_persist::{
        binary_persist_iota_bridge_req::IotaBridgeRequestParts,
        BinaryPersist,
        LinkedMessage,
        TangleMessageCompressed
    },
    http::{
        http_tools::{
            RequestBuilderTools,
            DispatchedRequestParts
        },
        http_protocol_streams::{
            URI_PREFIX_STREAMS,
            EndpointUris
        }
    }
};

pub(crate) async fn decode(external_id: &String, user_store: &UserDataStore, messages: &MessagesState, payload: Vec<u8>) -> Result<Message> {
    run_worker_in_own_thread::<DecodeWorker>(DecodeWorkerOptions::new(
        external_id,
        user_store,
        messages,
        payload,
    )).await
}

#[derive(Clone)]
struct DecodeWorkerOptions {
    external_id: String,
    user_store: UserDataStore,
    payload: Vec<u8>,
    multi_channel_mngr_opt: MultiChannelManagerOptions,
}

impl DecodeWorkerOptions {
    pub fn new(external_id: &String, user_store: &UserDataStore, messages: &MessagesState, payload: Vec<u8>) -> DecodeWorkerOptions {
        DecodeWorkerOptions{
            external_id: external_id.clone(),
            user_store: user_store.clone(),
            multi_channel_mngr_opt: messages.as_multi_channel_manager_options(),
            payload,
        }
    }
}

struct DecodeWorker;

#[async_trait(?Send)]
impl Worker for DecodeWorker {
    type OptionsType = DecodeWorkerOptions;
    type ResultType = Message;

    async fn run(opt: DecodeWorkerOptions) -> Result<Message> {
        let request_parts = match IotaBridgeRequestParts::try_from_bytes(opt.payload.as_slice()) {
            Ok(req) => req,
            Err(err) => {
                return Err(
                    AppError::GenericWithMessage(
                        StatusCode::BAD_REQUEST,
                        format!("Parsing the payload resulted in error: {}", err)
                    ));
            }
        };
        let hyper_request = request_parts.into_request(RequestBuilderTools::get_request_builder())
            .map_err(|e| AppError::GenericWithMessage(
                StatusCode::BAD_REQUEST,
                format!("The lorawan-rest request contained in the payload could not be converted into a hyper_request. Error: {}", e)
            ))?;

        let dispatch_req_parts = DispatchedRequestParts::new(hyper_request).await?;
        Self::decode_dispatch_req_parts(opt, dispatch_req_parts).await
    }
}

impl DecodeWorker {
    async fn decode_dispatch_req_parts(opt: DecodeWorkerOptions, req_parts: DispatchedRequestParts) -> Result<Message> {
        if req_parts.path.starts_with(URI_PREFIX_STREAMS) {
            match (&req_parts.method, req_parts.path.as_str()) {

                (&Method::POST, EndpointUris::SEND_MESSAGE) => {
                    let tangle_msg: LinkedMessage = LinkedMessage::try_from_bytes(&req_parts.binary_body).unwrap();
                    Self::decode_message(opt, tangle_msg.link.relative()).await
                },

                (&Method::POST, EndpointUris::SEND_COMPRESSED_MESSAGE) => {
                    let compressed_tangle_msg: TangleMessageCompressed = TangleMessageCompressed::try_from_bytes(&req_parts.binary_body).unwrap();
                    Self::decode_message(opt, compressed_tangle_msg.link.msgid).await
                },

                _ => {
                    return Err(
                        AppError::GenericWithMessage(
                            StatusCode::BAD_REQUEST,
                            format!("The lorawan-rest request contained in the payload addressed a REST endpoint that does not send a message. Request: {}", req_parts)
                        ));
                }
            }
        } else {
            Err(
                AppError::GenericWithMessage(
                    StatusCode::BAD_REQUEST,
                    format!("The lorawan-rest request contained in the payload addressed a REST endpoint that does not interact with a streams channel. Request: {}", req_parts)
                ))
        }
    }

    async fn decode_message(opt: DecodeWorkerOptions, msg_id: MsgId) -> Result<Message> {
        let mut channel_manager = match get_channel_manager_for_external_id(
            &opt.external_id,
            &opt.user_store,
            &opt.multi_channel_mngr_opt,
        ).await {
            Ok(mngr) => mngr,
            Err(_) => {
                return Err(
                    AppError::GenericWithMessage(
                        StatusCode::NOT_FOUND,
                        format!("A node with the external_id {} does not exist", opt.external_id)
                    ));
            }
        };

        if let Some(announcement_link) = channel_manager.announcement_link {
            let address = Address::new(announcement_link.base(), msg_id);
            if let Some(author) = channel_manager.user.as_mut() {
                if let Ok(unwrapped_msg) = author.receive_message(address).await {
                    Ok(unwrapped_msg.into())
                } else {
                    Err(AppError::GenericWithMessage(
                        StatusCode::NOT_FOUND,
                        format!("Could not receive message {} from tangle", address.to_string())
                    ))
                }
            } else {
                Err(AppError::ChannelDoesNotExist(address.base().to_string()))
            }

        } else {
            Err(AppError::GenericWithMessage(
                StatusCode::BAD_REQUEST,
                format!("Could not find an announcement message for external_id {}", opt.external_id)
            ))
        }
    }
}