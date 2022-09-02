use hyper::{
    Body,
    http::{
        Request,
        Response,
        Result,
        status,
    }
};

use super::{
    http_protocol_streams::{
        ServerDispatchStreams,
        dispatch_request_streams,
    },
    http_protocol_command::{
        ServerDispatchCommand,
        dispatch_request_command,
    },
    http_protocol_confirm::{
        ServerDispatchConfirm,
        dispatch_request_confirm,
    },
    http_protocol_lorawan_rest::{
        ServerDispatchLorawanRest,
        dispatch_request_lorawan_rest
    },
    http_tools::DispatchedRequestParts,
};

use crate::http::http_tools::{DispatchedRequestStatus, get_response_500, get_response_404};
use crate::binary_persist::binary_persist_iota_bridge_req::IotaBridgeResponseParts;

pub async fn dispatch_request(
    req: Request<Body>,
    lorawan_rest_callbacks: &mut impl ServerDispatchLorawanRest,
    streams_callbacks: &mut impl ServerDispatchStreams,
    command_callbacks: &mut impl ServerDispatchCommand,
    confirm_callbacks: &mut impl ServerDispatchConfirm,
) -> Result<Response<Body>> {
    let ret_val: Response<Body>;
    if let Ok( req_parts) = DispatchedRequestParts::new(req).await {
        if req_parts.path.starts_with(lorawan_rest_callbacks.get_uri_prefix()) {
            ret_val = dispatch_lorawan_rest_request(
                &req_parts,
                lorawan_rest_callbacks,
                streams_callbacks,
                command_callbacks,
                confirm_callbacks
            ).await?;
        } else {

            ret_val = dispatch_normal_request(req_parts, streams_callbacks, command_callbacks, confirm_callbacks).await?;
        }
    } else {
        log::debug!("[dispatch_request] Could not create DispatchedRequestParts from hyper request. Returning 500");
        ret_val = get_response_500()?;
    }

    Ok(ret_val)
}

async fn dispatch_lorawan_rest_request(
    req_parts: &DispatchedRequestParts,
    lorawan_rest_callbacks: &mut impl ServerDispatchLorawanRest,
    streams_callbacks: &mut impl ServerDispatchStreams,
    command_callbacks: &mut impl ServerDispatchCommand,
    confirm_callbacks: &mut impl ServerDispatchConfirm,
) -> Result<Response<Body>> {
    match dispatch_request_lorawan_rest(&req_parts, lorawan_rest_callbacks).await {
        Ok(lorawan_rest_request) => {
            match lorawan_rest_request.status {
                DispatchedRequestStatus::DeserializedLorawanRest => {
                    let response = dispatch_normal_request(
                        lorawan_rest_request,
                        streams_callbacks,
                        command_callbacks,
                        confirm_callbacks).await?;
                    let response_parts = IotaBridgeResponseParts::from_hyper_response(response).await;
                    response_parts.persist_to_hyper_response_200()
                }
                DispatchedRequestStatus::LorawanRest404 => {
                    get_response_404()
                },
                _ => {
                    log::debug!("[dispatch_lorawan_rest_request] Unexpected DispatchedRequestStatus: '{}'. Returning 500", lorawan_rest_request.status);
                    get_response_500()
                }
            }
        },
        Err(e) => {
            log::error!("[dispatch_lorawan_rest_request] Fatal error on dispatching lorawan rest request. Returning 500. Error is: {}", e);
            get_response_500()
        }
    }
}


pub async fn dispatch_normal_request(
    req_parts: DispatchedRequestParts,
    streams_callbacks: &mut impl ServerDispatchStreams,
    command_callbacks: &mut impl ServerDispatchCommand,
    confirm_callbacks: &mut impl ServerDispatchConfirm,
) -> Result<Response<Body>> {
    let mut ret_val: Option<Response<Body>> = None;
    log::debug!("[dispatch_normal_request] Dispatching request.path '{}'", req_parts.path);
    if req_parts.path.starts_with(streams_callbacks.get_uri_prefix()) {
        ret_val = Some(dispatch_request_streams(&req_parts, streams_callbacks).await?);
    }
    else if req_parts.path.starts_with(command_callbacks.get_uri_prefix()) {
        ret_val = Some(dispatch_request_command(&req_parts, command_callbacks).await?);
    }
    else if req_parts.path.starts_with(confirm_callbacks.get_uri_prefix()) {
        ret_val = Some(dispatch_request_confirm(&req_parts, confirm_callbacks).await?);
    }

    ret_val.ok_or_else(|| {
        if let Err(e) = status::StatusCode::from_u16(404) {
            e.into()
        } else {
            panic!("Should never happen");
        }
    })
}