use embedded_svc::{
    io::{
        Write,
        Read,
    },
    http::{
        Method,
        client::{
            Client,
            Request,
            Response,
        }
    }
};

use esp_idf_svc::{
    http::client::{
        EspHttpConnection,
        Configuration as HttpConfiguration,
    },
    errors::EspIOError,
};

use hyper::{
    body as hyper_body,
    body::Bytes,
    Body,
    Request as HyperRequest,
    http::{
        Method as HyperMethod,
        StatusCode,
    }
};

use anyhow::{
    Result,
    bail,
};
use std::{
    fmt,
    fmt::Formatter,
};

pub type EspHttpResponse<'a> = Response<&'a mut EspHttpConnection>;

const READ_STREAM_SUB_BUFFER_CAPACITY_SIZE: usize = 256;

pub fn read_stream_into_buffer<R: Read>(r: &mut R, buf: &mut Vec<u8>) -> Result<usize>{
    let mut internal_buffers = Vec::<[u8; READ_STREAM_SUB_BUFFER_CAPACITY_SIZE]>::new();
    let size_of_last_sub_buffer: usize;
    loop {
        let mut buffer = [0u8; READ_STREAM_SUB_BUFFER_CAPACITY_SIZE];
        let read_result = r.read(&mut buffer);
        internal_buffers.push(buffer);
        log::debug!("[fn read_stream_into_buffer()] Pushed buffer into internal_buffers vec. internal_buffers-length now is: {}", internal_buffers.len());
        match read_result {
            Ok(bytes_read) => {
                log::debug!("[read_stream_into_buffer] Reading bytes into buffer finished: Length: {}", bytes_read);
                if bytes_read < READ_STREAM_SUB_BUFFER_CAPACITY_SIZE {
                    log::debug!("[fn read_stream_into_buffer()] bytes_read < READ_STREAM_SUB_BUFFER_CAPACITY_SIZE. size_of_last_sub_buffer is set to {}", bytes_read);
                    size_of_last_sub_buffer = bytes_read;
                    break;
                }
            }
            Err(_) => {
                bail!("read call failed: Infallible")
            }
        }
    }

    let buf_size = (internal_buffers.len() - 1) * READ_STREAM_SUB_BUFFER_CAPACITY_SIZE + size_of_last_sub_buffer;
    log::debug!("[fn read_stream_into_buffer] Calculated over all buf_size is {} bytes", buf_size);
    buf.clear();
    let additional_capacity_needed = buf_size as i32 - buf.capacity() as i32;
    log::debug!("[fn read_stream_into_buffer] additional_capacity_needed is {} bytes", additional_capacity_needed);
    if additional_capacity_needed > 0 {
        buf.reserve(additional_capacity_needed as usize);
    }
    unsafe { buf.set_len(buf_size); }
    log::debug!("[fn read_stream_into_buffer] Finished buf.set_len({})", buf_size);

    let pos_of_last_sub_buffer = internal_buffers.len() - 1;
    log::debug!("[fn read_stream_into_buffer] pos_of_last_sub_buffer in the internal_buffers vec is {}", pos_of_last_sub_buffer);
    for (pos, sub_buffer) in internal_buffers.iter().enumerate() {
        let src_size = if pos < pos_of_last_sub_buffer {READ_STREAM_SUB_BUFFER_CAPACITY_SIZE} else {size_of_last_sub_buffer};
        log::debug!("[fn read_stream_into_buffer] Processing temporary sub_buffer with index {} in internal_buffers vec. Buffers src_size is {}", pos, src_size);
        let dst_start = pos * READ_STREAM_SUB_BUFFER_CAPACITY_SIZE;
        let dst_end = dst_start + src_size;
            log::debug!("[fn read_stream_into_buffer] Cloning slice of {} bytes from {} to {} into buffer handed to this function",
                    (dst_end - dst_start),
                    dst_start,
                    dst_end
        );
        buf[dst_start..dst_end].clone_from_slice(&sub_buffer[..src_size]);
    }
    Ok(buf_size)
}

pub enum UserAgentName {
    Main,
    CommandFetcher,
    LoraWanRestClient
}

impl UserAgentName {
    pub const COMMAND_FETCHER: &'static str = "sensor/command-fetcher";
    pub const MAIN: &'static str = "sensor/main";
    pub const LORAWAN_REST_CLIENT: &'static str = "sensor/lorawan-rest-client";

    pub fn value(&self) -> &'static str {
        match self {
            UserAgentName::Main => UserAgentName::MAIN,
            UserAgentName::CommandFetcher => UserAgentName::COMMAND_FETCHER,
            UserAgentName::LoraWanRestClient => UserAgentName::LORAWAN_REST_CLIENT,
        }
    }
}

const USER_AGENT_KEY: &'static str = "user-agent";

type Esp32KeyValuePair<'a> = (&'a str, &'a str);

struct Esp32HttpHeaderTool {}

impl Esp32HttpHeaderTool {
    pub fn get_content_len_string(method: &Method, content_len: usize) -> String {
        let mut content_len_string = "0".to_string();
        if method == &Method::Post {
            log::debug!("[HyperEsp32Client.send] pre setting content_len_value");
            content_len_string = content_len.to_string();
        }
        content_len_string
    }

    pub fn create_esp32_http_headers<'a>(user_agent_name: &UserAgentName, content_len_str: &'a str) -> Vec<Esp32KeyValuePair<'a>> {
        vec![
            (USER_AGENT_KEY, user_agent_name.value()),
            ("content-type", "application/octet-stream"),
            ("content-length", content_len_str),
            ("connection", "close")
        ]
    }
}

pub struct HyperEsp32Client {
    http_client: Client<EspHttpConnection>,
    user_agent_name: UserAgentName,
    url: String,
}

pub struct SimpleHttpResponse {
    pub status: StatusCode,
    pub body: Vec<u8>,
}

impl fmt::Display for SimpleHttpResponse {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Http Response: status: {}. Body length: {}", self.status, self.body.len())
    }
}

impl HyperEsp32Client {

    pub fn new(http_configuration: &HttpConfiguration, user_agent_name: UserAgentName ) -> Self {
        Self {
            http_client: Client::wrap(
                EspHttpConnection::new(http_configuration).expect("[HyperEsp32Client.new] Error on creating EspHttpConnection")
            ),
            user_agent_name,
            url: "".to_string()
        }
    }

    pub async fn send(&mut self, req: HyperRequest<Body>) -> Result<SimpleHttpResponse> {
        log::debug!("[HyperEsp32Client.send] Start");
        self.url = req.uri().to_string();

        let svc_http_method = HyperEsp32Client::get_svc_http_method(&req);
        let bytes = hyper_body::to_bytes(req.into_body()).await.unwrap();

        log::debug!("[HyperEsp32Client.send] hyper_body::to_bytes");
        let content_len_string = Esp32HttpHeaderTool::get_content_len_string(&svc_http_method, bytes.len());
        log::debug!("[HyperEsp32Client.send] content_len_string is {}", content_len_string);
        let http_header = Esp32HttpHeaderTool::create_esp32_http_headers(
            &self.user_agent_name,
            content_len_string.as_str()
        );
        log::debug!("[HyperEsp32Client.send] Created http_header");

        let mut esp_http_req = self.http_client.request(
            svc_http_method,
            self.url.as_str(),
            http_header.as_slice()
        )?;

        HyperEsp32Client::prepare_esp_http_req(svc_http_method, &bytes, &mut esp_http_req)?;
        log::debug!("[HyperEsp32Client.send] Prepared esp_http_req");
        let resulting_response = esp_http_req.submit();
        log::debug!("[HyperEsp32Client.send] Submitted esp_http_req");

        HyperEsp32Client::handle_resulting_response(resulting_response)
    }

    fn handle_resulting_response(resulting_response: Result<Response<&mut EspHttpConnection>, EspIOError>) -> Result<SimpleHttpResponse> {
        match resulting_response {
            Ok(mut resp) => {
                log::debug!("[HyperEsp32Client.handle_resulting_response] Received EspHttpResponse");
                let (_headers, mut body) = resp.split();
                let mut buffer = Vec::new();
                read_stream_into_buffer(&mut body, &mut buffer)?;

                Ok(SimpleHttpResponse {
                    status: StatusCode::from_u16(resp.status())?,
                    body: buffer
                })
            },
            Err(e) => {
                log::error!("espHttpReq.submit failed: {}", e);
                bail!("espHttpReq.submit failed: {}", e)
            }
        }
    }

    fn prepare_esp_http_req(svc_http_method: Method, bytes: &Bytes, esp_http_req: &mut Request<&mut EspHttpConnection>) -> Result<()>{
        match svc_http_method {
            Method::Post => {
                log::debug!("[HyperEsp32Client.prepare_esp_http_req] Bytes to send: Length: {}\n    {:02X?}", bytes.len(), bytes);
                esp_http_req.write_all(&bytes)?;
                log::debug!("[HyperEsp32Client.prepare_esp_http_req] Sending bytes was successful");
                esp_http_req.flush()?;
                log::debug!("[HyperEsp32Client.prepare_esp_http_req] Flushing esp_http_req was successful");
            },
            Method::Get => {},
            _ => {
                bail!("svc_http_method is currently not supported")
            },
        }
        Ok(())
    }

    fn get_svc_http_method(req: &HyperRequest<Body>) -> Method {
        let svc_http_method = match req.method() {
            &HyperMethod::POST => Method::Post,
            &HyperMethod::GET => Method::Get,
            _ => Method::Unbind,
        };
        log::debug!("[HyperEsp32Client.get_svc_http_method] svc_http_method found");
        svc_http_method
    }
}


