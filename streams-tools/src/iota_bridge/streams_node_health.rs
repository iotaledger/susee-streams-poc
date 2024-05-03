#![allow(non_snake_case)]

use hyper::{
    Client as HyperClient,
    Body,
    client::HttpConnector,
    http::{
        Request,
        StatusCode,
    }
};

use anyhow::{
    Result,
    anyhow,
};

use crate::{
    helpers::get_iota_node_url,
    http::http_tools::RequestBuilderTools,
    streams_transport::streams_transport::{
        STREAMS_TOOLS_CONST_INX_COLLECTOR_PORT,
        STREAMS_TOOLS_CONST_MINIO_DB_PORT
    }
};

#[derive(Clone)]
pub struct HealthCheckerOptions {
    pub iota_node: String,
    pub inx_collector_port: u16,
    pub minio_db_poort: u16,
}

impl HealthCheckerOptions {
    pub fn new(iota_node: String) -> Self {
        let mut ret_val = Self::default();
        ret_val.iota_node = iota_node;
        ret_val
    }

    pub fn get_inx_collector_url(&self) -> String {
        format!("http://{}:{}", self.iota_node, self.inx_collector_port)
    }

    pub fn get_minio_db_url(&self) -> String {
        format!("http://{}:{}", self.iota_node, self.minio_db_poort)
    }

    pub fn get_iota_node_url(&self) -> String {
        get_iota_node_url(self.iota_node.as_str())
    }
}

impl Default for HealthCheckerOptions {
    fn default() -> Self {
        Self {
            iota_node: "127.0.0.1".to_string(),
            inx_collector_port: STREAMS_TOOLS_CONST_INX_COLLECTOR_PORT,
            minio_db_poort: STREAMS_TOOLS_CONST_MINIO_DB_PORT,
        }
    }
}

#[derive(Clone)]
pub struct HealthChecker {
    hyper_client: HyperClient<HttpConnector, Body>,
    options: HealthCheckerOptions,
}

struct EndpointUris {}

impl EndpointUris {
    pub const INX_COLLECTOR_BLOCK: &'static str = "/block";
    pub const MINIO_HEALTH: &'static str = "/minio/health/live";
    pub const IOTA_NODE_HEALTH: &'static str = "/health";


    pub fn get_uri___inx_collector___get_not_existing_block_tag() -> String {
        format!("{}/{}?checkExistence=true", Self::INX_COLLECTOR_BLOCK, "not-existing-block")
    }

    pub fn get_uri___minio_db___health() -> String {
        Self::MINIO_HEALTH.to_string()
    }

    pub fn get_uri___iota_node___health() -> String {
        Self::IOTA_NODE_HEALTH.to_string()
    }
}

impl HealthChecker {
    pub fn new(options: HealthCheckerOptions) -> HealthChecker {
        HealthChecker {
            hyper_client: HyperClient::new(),
            options,
        }
    }

    pub async fn is_healthy(&self) -> Result<bool> {
        let iota_node_url = format!("{}{}",
            self.options.get_iota_node_url(),
            EndpointUris::get_uri___iota_node___health()
        );
        log::debug!("[fn is_healthy()] iota_node_url: {}", iota_node_url);
        if !self.is_request_successful(iota_node_url, "IOTA Node", None).await? {
            return Ok(false);
        }

        let inx_collector_url = format!("{}{}",
            self.options.get_inx_collector_url(),
            EndpointUris::get_uri___inx_collector___get_not_existing_block_tag()
        );
        log::debug!("[fn is_healthy()] inx_collector_url: {}", inx_collector_url);
        if !self.is_request_successful(inx_collector_url, "INX Collector",Some(StatusCode::BAD_REQUEST)).await? {
            return Ok(false);
        }

        let minio_url = format!("{}{}",
                                        self.options.get_minio_db_url(),
                                        EndpointUris::get_uri___minio_db___health()
        );
        log::debug!("[fn is_healthy()] minio_url: {}", minio_url);
        if !self.is_request_successful(minio_url, "Minio", None).await? {
            return Ok(false);
        }

        Ok(true)
    }

    async fn is_request_successful(&self, uri: String, tested_service: &str, additional_allowed_status: Option<StatusCode>) -> Result<bool> {
        let request = self.get_request(uri)?;
        match self.hyper_client.request(request).await {
            Ok(resp) => {
                if let Some(allowed_status) = additional_allowed_status {
                    if resp.status() == allowed_status {
                        return Ok(true);
                    }
                }
                match resp.status() {
                    StatusCode::OK => Ok(true),
                    _ => Ok(false)
                }
            },
            Err(e) => {
                log::error!("[fn is_request_successful()] hyper_client returned error for {} service: {}", tested_service, e);
                Ok(false)
            }
        }
    }

    fn get_request(&self, uri: String) -> Result<Request<Body>> {
        RequestBuilderTools::get_request_builder()
            .method("GET")
            .uri(uri)
            .body(Body::empty())
            .map_err(|e| anyhow!(e))
    }
}