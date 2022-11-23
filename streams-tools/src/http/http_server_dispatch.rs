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
use crate::http::http_protocol_lorawan_node::{ServerDispatchLoraWanNode, dispatch_request_lorawan_node};

pub struct NormalDispatchCallbacks<'a, Streams, Command, Confirm, LorawanNode>
    where
        Streams: ServerDispatchStreams,
        Command: ServerDispatchCommand,
        Confirm: ServerDispatchConfirm,
        LorawanNode: ServerDispatchLoraWanNode,
{
    pub streams: &'a mut Streams,
    pub command: &'a mut Command,
    pub confirm: &'a mut Confirm,
    pub lorawan_node: &'a mut LorawanNode,
}

pub async fn dispatch_request<'a, Streams, Command, Confirm, LorawanNode>(
    req: Request<Body>,
    lorawan_rest_callbacks: &mut impl ServerDispatchLorawanRest,
    normal_callbacks: &mut NormalDispatchCallbacks<'a, Streams, Command, Confirm, LorawanNode>,
) -> Result<Response<Body>>
    where
        Streams: ServerDispatchStreams,
        Command: ServerDispatchCommand,
        Confirm: ServerDispatchConfirm,
        LorawanNode: ServerDispatchLoraWanNode,
{
    let ret_val: Response<Body>;
    if let Ok( req_parts) = DispatchedRequestParts::new(req).await {
        if req_parts.path.starts_with(lorawan_rest_callbacks.get_uri_prefix()) {
            ret_val = dispatch_lorawan_rest_request(
                &req_parts,
                lorawan_rest_callbacks,
                normal_callbacks
            ).await?;
        } else {
            ret_val = normal_callbacks.dispatch(req_parts).await?;
        }
    } else {
        log::debug!("[dispatch_request] Could not create DispatchedRequestParts from hyper request. Returning 500");
        ret_val = get_response_500("Error on initial deserialization of your request")?;
    }

    Ok(ret_val)
}

async fn dispatch_lorawan_rest_request<'a, Streams, Command, Confirm, LorawanNode>(
    req_parts: &DispatchedRequestParts,
    lorawan_rest_callbacks: &mut impl ServerDispatchLorawanRest,
    normal_callbacks: &mut NormalDispatchCallbacks<'a, Streams, Command, Confirm, LorawanNode>,
) -> Result<Response<Body>>
    where
        Streams: ServerDispatchStreams,
        Command: ServerDispatchCommand,
        Confirm: ServerDispatchConfirm,
        LorawanNode: ServerDispatchLoraWanNode,
{
    match dispatch_request_lorawan_rest(&req_parts, lorawan_rest_callbacks).await {
        Ok(req_parts_inner) => {
            match req_parts_inner.status {
                DispatchedRequestStatus::DeserializedLorawanRest => {
                    log::debug!("[dispatch_lorawan_rest_request] Processing DeserializedLorawanRest now");
                    let response = normal_callbacks.dispatch(req_parts_inner).await?;
                    let response_parts = IotaBridgeResponseParts::from_hyper_response(response).await;
                    response_parts.persist_to_hyper_response_200()
                }
                DispatchedRequestStatus::LorawanRest404 => {
                    get_response_404("The lorawan-rest API function addressed by the requested URL does not exist")
                },
                _ => {
                    log::debug!("[dispatch_lorawan_rest_request] Unexpected DispatchedRequestStatus: '{}'. Returning 500", req_parts_inner.status);
                    get_response_500("The lorawan-rest request resulted in an unexpected status")
                }
            }
        },
        Err(e) => {
            log::error!("[dispatch_lorawan_rest_request] Fatal error on dispatching lorawan rest request. Returning 500. Error is: {}", e);
            get_response_500("Error on deserialization of your lorawan-rest request")
        }
    }
}

impl<'a, Streams, Command, Confirm, LorawanNode> NormalDispatchCallbacks<'a, Streams, Command, Confirm, LorawanNode>
    where
        Streams: ServerDispatchStreams,
        Command: ServerDispatchCommand,
        Confirm: ServerDispatchConfirm,
        LorawanNode: ServerDispatchLoraWanNode,
{

    pub async fn dispatch(&mut self, req_parts: DispatchedRequestParts) -> Result<Response<Body>> {
        let mut ret_val: Option<Response<Body>> = None;
        log::debug!("[dispatch_normal_request] Dispatching request.path '{}'", req_parts.path);
        if req_parts.path.starts_with(self.streams.get_uri_prefix()) {
            ret_val = Some(dispatch_request_streams(&req_parts, self.streams).await?);
        }
        else if req_parts.path.starts_with(self.command.get_uri_prefix()) {
            ret_val = Some(dispatch_request_command(&req_parts, self.command).await?);
        }
        else if req_parts.path.starts_with(self.confirm.get_uri_prefix()) {
            ret_val = Some(dispatch_request_confirm(&req_parts, self.confirm).await?);
        }
        else if req_parts.path.starts_with(self.lorawan_node.get_uri_prefix()) {
            ret_val = Some(dispatch_request_lorawan_node(&req_parts, self.lorawan_node).await?);
        }

        log::debug!("[dispatch_normal_request] Exiting function");
        ret_val.ok_or_else(|| {
            if let Err(e) = status::StatusCode::from_u16(404) {
                log::debug!("[dispatch_normal_request] ret_val is Err. Returning 404");
                e.into()
            } else {
                panic!("Should never happen");
            }
        })
    }
}