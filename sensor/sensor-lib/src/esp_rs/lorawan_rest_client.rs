use crate::esp_rs::hyper_esp_rs_tools::{HyperEsp32Client, UserAgentName, SimpleHttpResponse};

use streams_tools::{
    http::http_protocol_lorawan_rest::RequestBuilderLorawanRest,
    LoraWanRestClientOptions,
};

use anyhow::Result;

pub struct LoraWanRestClient {
    http_client: HyperEsp32Client,
    request_builder: RequestBuilderLorawanRest,
}

impl<'a> LoraWanRestClient {
    pub fn new(options: Option<LoraWanRestClientOptions<'a>>) -> Self {
        let options = options.unwrap_or_default();
        log::debug!("[fn new()] Initializing instance with options:\n       {}\n", options);
        Self {
            http_client: HyperEsp32Client::new(&Default::default(), UserAgentName::LoraWanRestClient),
            request_builder: RequestBuilderLorawanRest::new(options.iota_bridge_url)
        }
    }

    pub async fn post_binary_request_to_iota_bridge(&mut self, request_bytes: Vec<u8>, dev_eui: &str) -> Result<SimpleHttpResponse> {
        log::debug!("[fn post_binary_request_to_iota_bridge()] Sending {} bytes via lora-wan-rest http request to iota-bridge", request_bytes.len());
        self.http_client.send(
            self.request_builder.post_binary_request(request_bytes, dev_eui)
                .expect("Error on building http requ\
                est for api function 'post_binary_request'")
        ).await
    }
}