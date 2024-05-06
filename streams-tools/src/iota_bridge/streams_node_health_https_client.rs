use http::{
    Request,
    StatusCode
};

use bytes::Bytes;

use http_body_util::Empty;

use hyper_tls::HttpsConnector;

use hyper_util::{
    client::legacy::{
        Client as HyperClient,
        connect::HttpConnector,
    },
    rt::TokioExecutor
};

use anyhow::{
    Result,
    anyhow,
};

#[derive(Clone)]
pub struct HttpsClient {
    hyper_client: HyperClient<HttpsConnector<HttpConnector>, Empty<bytes::Bytes>>,
}

impl HttpsClient {
    pub fn new() -> HttpsClient {
        let https = HttpsConnector::new();
        let hyper_client = HyperClient::builder(TokioExecutor::new())
            .build::<_, Empty<Bytes>>(https);
        HttpsClient {
            hyper_client,
        }
    }

    pub(crate) async fn is_request_successful(&self, uri: String, tested_service: &str, additional_allowed_status: Option<StatusCode>) -> Result<bool> {
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

    fn get_request(&self, uri: String) -> Result<Request<Empty::<bytes::Bytes>>> {
        Request::builder().header("User-Agent", "iota-bridge/1.0")
            .method("GET")
            .uri(uri)
            .body(Empty::<bytes::Bytes>::new())
            .map_err(|e| anyhow!(e))
    }
}