use std::rc::Rc;

use hyper::{
    Body,
    http::{
        Request,
        Response,
        Result,
        status,
    }
};

use crate::{
    binary_persist::binary_persist_iota_bridge_req::IotaBridgeResponseParts,
};

use super::{
    ServerProcessFinally,
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
        dispatch_request_lorawan_rest,
        translate_lorawan_rest_error,
    },
    http_protocol_lorawan_node::{
        ServerDispatchLoraWanNode,
        dispatch_request_lorawan_node
    },
    http_tools::{
        DispatchedRequestParts,
        DispatchedRequestStatus,
        get_response_500,
        get_response_404
    },
    http_dispatch_scope::{
        DispatchScope,
        ScopeProvide
    },
};

pub struct NormalDispatchCallbacks<'a, Scope, Streams, Command, Confirm, LorawanNode, Finally>
    where
        Scope: ScopeProvide,
        Streams: ServerDispatchStreams,
        Command: ServerDispatchCommand,
        Confirm: ServerDispatchConfirm,
        LorawanNode: ServerDispatchLoraWanNode,
        Finally: ServerProcessFinally,
{
    pub scope_provide: &'a mut Scope,
    pub streams: &'a mut Streams,
    pub command: &'a mut Command,
    pub confirm: &'a mut Confirm,
    pub lorawan_node: &'a mut LorawanNode,
    pub finally: &'a mut Finally,
}

pub async fn dispatch_request<'a, Scope, Streams, Command, Confirm, LorawanNode, Finally>(
    req: Request<Body>,
    lorawan_rest_callbacks: &mut impl ServerDispatchLorawanRest,
    normal_callbacks: &mut NormalDispatchCallbacks<'a, Scope, Streams, Command, Confirm, LorawanNode, Finally>,
) -> Result<Response<Body>>
    where
        Scope: ScopeProvide,
        Streams: ServerDispatchStreams,
        Command: ServerDispatchCommand,
        Confirm: ServerDispatchConfirm,
        LorawanNode: ServerDispatchLoraWanNode,
        Finally: ServerProcessFinally,
{
    let mut ret_val: Response<Body>;
    let scope = normal_callbacks.create_new_scope();
    if let Ok( req_parts) = DispatchedRequestParts::new(req).await {
        if req_parts.path.starts_with(lorawan_rest_callbacks.get_uri_prefix()) {
            lorawan_rest_callbacks.set_scope(scope.clone());
            ret_val = dispatch_lorawan_rest_request(
                &req_parts,
                lorawan_rest_callbacks,
                normal_callbacks
            ).await?;
        } else {
            ret_val = normal_callbacks.dispatch(&req_parts).await?;
        }

        ret_val = normal_callbacks.finally.process(ret_val, &req_parts, scope.as_ref()).await?;
    } else {
        log::debug!("[fn dispatch_request()] Could not create DispatchedRequestParts from hyper request. Returning 500");
        ret_val = get_response_500("Error on initial deserialization of your request")?;
    }

    Ok(ret_val)
}

async fn dispatch_lorawan_rest_request<'a, Scope, Streams, Command, Confirm, LorawanNode, Finally>(
    req_parts: &DispatchedRequestParts,
    lorawan_rest_callbacks: &mut impl ServerDispatchLorawanRest,
    normal_callbacks: &mut NormalDispatchCallbacks<'a, Scope, Streams, Command, Confirm, LorawanNode, Finally>,
) -> Result<Response<Body>>
    where
        Scope: ScopeProvide,
        Streams: ServerDispatchStreams,
        Command: ServerDispatchCommand,
        Confirm: ServerDispatchConfirm,
        LorawanNode: ServerDispatchLoraWanNode,
        Finally: ServerProcessFinally,
{
    match dispatch_request_lorawan_rest(&req_parts, lorawan_rest_callbacks).await {
        Ok(req_parts_inner) => {
            match req_parts_inner.status {
                DispatchedRequestStatus::DeserializedLorawanRest => {
                    log::debug!("[fn dispatch_request_lorawan_rest()] Processing DeserializedLorawanRest now");
                    let response = normal_callbacks.dispatch(&req_parts_inner).await?;
                    let response_status = translate_lorawan_rest_error(response.status());
                    let response_parts = IotaBridgeResponseParts::from_hyper_response(response).await;
                    log::info!("[dispatch_request_lorawan_rest] DevEUI: {} - Returning response {} for lorawan_rest request:\n{}",
                               req_parts_inner.dev_eui,
                               response_status,
                               response_parts
                    );
                    response_parts.persist_to_hyper_response(response_status)
                }
                DispatchedRequestStatus::LorawanRest404 => {
                    get_response_404("The lorawan-rest API function addressed by the requested URL does not exist")
                },
                _ => {
                    log::debug!("[fn dispatch_request_lorawan_rest()] Unexpected DispatchedRequestStatus: '{}'. Returning 500", req_parts_inner.status);
                    get_response_500("The lorawan-rest request resulted in an unexpected status")
                }
            }
        },
        Err(e) => {
            log::error!("[fn dispatch_request_lorawan_rest()] Fatal error on dispatching lorawan rest request. Returning 500. Error is: {}", e);
            get_response_500("Error on deserialization of your lorawan-rest request")
        }
    }
}

impl<'a, Scope, Streams, Command, Confirm, LorawanNode, Finally> NormalDispatchCallbacks<'a, Scope, Streams, Command, Confirm, LorawanNode, Finally>
    where
        Scope: ScopeProvide,
        Streams: ServerDispatchStreams,
        Command: ServerDispatchCommand,
        Confirm: ServerDispatchConfirm,
        LorawanNode: ServerDispatchLoraWanNode,
        Finally: ServerProcessFinally,
{
    pub async fn dispatch(&mut self, req_parts: &DispatchedRequestParts) -> Result<Response<Body>> {
        let mut ret_val: Option<Response<Body>> = None;
        log::debug!("[NormalDispatchCallbacks.dispatch] Dispatching request.path '{}'", req_parts.path);
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

        log::debug!("[NormalDispatchCallbacks.dispatch] Exiting function");
        if ret_val.is_none() {
            log::debug!("[NormalDispatchCallbacks.dispatch] ret_val is None. Returning 404");
            ret_val = Some( Response::builder()
                .status(status::StatusCode::NOT_FOUND)
                .body(Default::default())?
            );
        }
        ret_val.ok_or_else(|| {
            panic!("[NormalDispatchCallbacks.dispatch] Should never happen");
        })
    }

    fn create_new_scope(&mut self) -> Rc<dyn DispatchScope> {
        let scope = self.scope_provide.create_new_scope();

        self.streams.set_scope(scope.clone());
        self.command.set_scope(scope.clone());
        self.confirm.set_scope(scope.clone());
        self.lorawan_node.set_scope(scope.clone());

        scope
    }
}