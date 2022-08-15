use embedded_svc::http::client::{
    Client,
    Request,
    RequestWrite
};

use esp_idf_svc::{
    http::client::{
        EspHttpClient,
        EspHttpResponse
    },
    errors::EspIOError
};

use hyper::{
    body as hyper_body,
    Body,
    Request as HyperRequest,
    http::{
        Method,
    }
};

use anyhow::{
    Result,
    bail,
};

pub async fn send_hyper_request_via_esp_http<'a>(http_client: &'a mut EspHttpClient, req: HyperRequest<Body>) -> Result<EspHttpResponse<'a>> {
    let svc_http_method = match req.method() {
        &Method::POST => embedded_svc::http::Method::Post,
        &Method::GET => embedded_svc::http::Method::Get,
        _ => embedded_svc::http::Method::Unbind,
    };

    let esp_http_req = http_client.request(
        svc_http_method,
        &req.uri().to_string(),
    )?;

    let resulting_ret_val: Result<EspHttpResponse, EspIOError>;
    match req.method() {
        &Method::POST => {
            let bytes = hyper_body::to_bytes(req.into_body()).await.unwrap();
            log::debug!("[HttpClient.request] Bytes to send: Length: {}\n    {:02X?}", bytes.len(), bytes);
            let http_request_write = esp_http_req.send_bytes(&bytes)?;
            resulting_ret_val = http_request_write.submit();
        },
        &Method::GET => {
            resulting_ret_val = esp_http_req.submit();
        },
        _ => {
            bail!("Method '{}' is currently not supported", req.method())
        },
    }

    match resulting_ret_val {
        Ok(resp) => {
            log::debug!("[send_hyper_request_via_esp_http] Received EspHttpResponse");
            Ok(resp)
        },
        Err(e) => {
            bail!("espHttpReq.submit failed: {}", e)
        }
    }
}
