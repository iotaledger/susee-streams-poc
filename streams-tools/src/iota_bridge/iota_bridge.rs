use iota_streams::{
    app::transport::tangle::client::Client,
};

use std::{
    clone::Clone,
};

use hyper::{
    Body,
    http::{
        Request,
        Response,
        Result,
    }
};
use crate::{
    iota_bridge::{
        DispatchStreams,
        DispatchCommand,
        DispatchConfirm,
        DispatchLorawanRest,
        LoraWanNodeDataStore,
    },
    http::dispatch_request
};

#[derive(Clone)]
pub struct IotaBridge<'a> {
    dispatch_streams: DispatchStreams,
    dispatch_command: DispatchCommand<'a>,
    dispatch_confirm: DispatchConfirm<'a>,
    dispatch_lorawan_rest: DispatchLorawanRest,
}

impl<'a> IotaBridge<'a>
{
    pub fn new(url: &str, lora_wan_node_store: LoraWanNodeDataStore) -> Self {
        let client = Client::new_from_url(url);

        Self {
            dispatch_streams: DispatchStreams::new(&client, lora_wan_node_store),
            dispatch_command: DispatchCommand::new(),
            dispatch_confirm: DispatchConfirm::new(),
            dispatch_lorawan_rest: DispatchLorawanRest::new(),
        }
    }

    pub async fn handle_request(&mut self, req: Request<Body>) -> Result<Response<Body>> {
        dispatch_request(req,
             &mut self.dispatch_lorawan_rest,
             &mut self.dispatch_streams,
             &mut self.dispatch_command,
             &mut self.dispatch_confirm
        ).await
    }
}
