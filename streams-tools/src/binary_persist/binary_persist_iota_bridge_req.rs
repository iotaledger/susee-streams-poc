use hyper::{
    Body, body,
    http::{
        Result,
        StatusCode,
        Response as HyperResponse,
        request::{
            Builder,
            Request,
        }
    },
    Method
};

use std::{
    fmt,
    fmt::Formatter,
    ops::Range,
    str::FromStr,
    ptr,
    ffi::{
        CStr,
        c_char,
    },
};

use crate::{
    STREAMS_TOOLS_CONST_DEV_EUI_NOT_DEFINED,
    binary_persist::{
        BinaryPersist,
        USIZE_LEN,
        RangeIterator,
        deserialize_string,
        serialize_vec_u8,
        deserialize_vec_u8,
        serialize_string,
    }
};

use anyhow::bail;

use bitflags::bitflags;

#[derive(Debug, Clone)]
pub enum HttpMethod {
    POST,
    GET,
}

impl fmt::Display for HttpMethod {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl FromStr for HttpMethod {
    type Err = anyhow::Error;

    fn from_str(method_str: &str) -> anyhow::Result<Self> {
        let upper_method_str = method_str.to_uppercase();
        match upper_method_str.as_str() {
            "GET" => Ok(HttpMethod::GET),
            "POST" => Ok(HttpMethod::POST),
            _ => bail!("'{}' is not a valid HttpMethod value", method_str)
        }
    }
}

impl From<HeaderFlags> for HttpMethod {
    fn from(header_flags: HeaderFlags) -> Self {
        if header_flags.contains(HeaderFlags::IS_METHOD_POST) && header_flags.contains(HeaderFlags::IS_METHOD_GET) {
            panic!("Severe Error: IS_METHOD_POST and IS_METHOD_GET are both set to true. These flags are mutually exclusive.")
        }
        if !header_flags.contains(HeaderFlags::IS_METHOD_POST) && !header_flags.contains(HeaderFlags::IS_METHOD_GET) {
            panic!("Severe Error: IS_METHOD_POST and IS_METHOD_GET are both set to false. One of both flags needs to be set.")
        }
        if header_flags.contains(HeaderFlags::IS_METHOD_POST) {
            Self::POST
        } else {
            Self::GET
        }
    }
}

impl From<&Method> for HttpMethod {
    fn from(method: &Method) -> Self {
        match method {
            &Method::POST => Self::POST,
            &Method::GET => Self::GET,
            _ => panic!("'{}' is not a valid HttpMethod value", method)
        }
    }
}

pub const HEADER_FLAGS_LEN: usize = 1;
pub type HeaderFlagsNumericalType = u8;

bitflags! {
    #[derive(Default)]
    pub struct HeaderFlags: HeaderFlagsNumericalType {
        const NEEDS_REGISTERED_LORAWAN_NODE = 0b00000001;
        const IS_METHOD_POST = 0b00000010;
        const IS_METHOD_GET = 0b00000100;
    }
}

impl From<HttpMethod> for HeaderFlags {
    fn from(method: HttpMethod) -> Self {
        match method {
            HttpMethod::POST => Self::IS_METHOD_POST,
            HttpMethod::GET => Self::IS_METHOD_GET,
        }
    }
}

#[derive(Clone)]
pub struct IotaBridgeRequestParts {
    pub method: HttpMethod,
    pub uri: String,
    pub body_bytes: Vec<u8>,
    uri_bytes: Vec<u8>,
    header_flags: HeaderFlags,
}

impl IotaBridgeRequestParts {
    pub fn new(header_flags: HeaderFlags, uri: String, body_bytes: Vec<u8>) -> Self {
        let method = HttpMethod::from(header_flags);
        let uri_bytes = uri.clone().into_bytes();
        Self {
            method,
            uri,
            body_bytes,
            uri_bytes,
            header_flags
        }
    }

    pub async fn from_request(request: Request<Body>, needs_registered_lorawan_node: bool) -> Self {
        let method = HttpMethod::from(request.method());
        let uri = request.uri().to_string();
        let body_bytes = body::to_bytes(request.into_body()).await.unwrap().to_vec();
        let uri_bytes = uri.clone().into_bytes();
        let mut header_flags = HeaderFlags::from(method.clone());
        header_flags.set(HeaderFlags::NEEDS_REGISTERED_LORAWAN_NODE, needs_registered_lorawan_node);
        Self {
            method,
            uri,
            body_bytes,
            uri_bytes,
            header_flags
        }
    }

    pub fn into_request(self: Self, request_builder: Builder) -> Result<Request<Body>> {
        let body = if self.body_bytes.len() == 0 {
            Body::empty()
        } else {
            Body::from(self.body_bytes)
        };

        request_builder
            .method(self.method.to_string().as_str())
            .uri(String::from(self.uri.as_str()))
            .body(body)
    }

    pub fn is_buffer_length_correct(buffer: &[u8], buffer_length: usize ) -> bool {
        let (buffer_length_is_correct, _, _) = is_request_buffer_length_correct(buffer, buffer_length);
        buffer_length_is_correct
    }

    pub fn get_request_byte_size(buffer: &[u8]) -> anyhow::Result<usize> {
        if buffer.len() < USIZE_LEN {
            bail!("The buffer length must be at least {} bytes", USIZE_LEN)
        }
        let (_, _, total_needed_size) = is_request_buffer_length_correct(buffer, buffer.len());
        Ok(total_needed_size)
    }

    pub fn needs_registerd_lorawan_node(&self) -> bool {
        self.header_flags.contains(HeaderFlags::NEEDS_REGISTERED_LORAWAN_NODE)
    }
}

impl fmt::Display for IotaBridgeRequestParts {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "IotaBridgeRequestParts: method: {}, uri: {}, body length: {}",
               self.method, self.uri, self.body_bytes.len())
    }
}

pub fn is_request_buffer_length_correct(buffer: &[u8], buffer_length: usize) -> (bool, Range<usize>, usize) {
    let range: Range<usize> = RangeIterator::new(USIZE_LEN);
    let total_needed_size = u32::try_from_bytes(&buffer[range.clone()]).unwrap() as usize;
    (buffer_length <= total_needed_size, range, total_needed_size)
}

impl BinaryPersist for IotaBridgeRequestParts {
    fn needed_size(&self) -> usize {
        // Request parts will be serialized in the the following order
        // as every Request part has a non static length we need 4 bytes to store the length for each part
        // # - Property                 - Byte size
        // ---------------------------------------------------------------
        // 1 - total needed buffer size - USIZE_LEN
        // 2 - header_flags             - HEADER_FLAGS_LEN
        // 3 - uri                      - USIZE_LEN + String length
        // 4 - body                     - USIZE_LEN + Vec length
        let length_values_size = 3 * USIZE_LEN;
        length_values_size + HEADER_FLAGS_LEN + self.uri_bytes.len() + self.body_bytes.len()
    }

    fn to_bytes(&self, buffer: &mut [u8]) -> anyhow::Result<usize> {
        if buffer.len() < self.needed_size() {
            panic!("[BinaryPersist for IotaBridgeRequestParts - to_bytes()] This Request needs {} bytes but \
                    the provided buffer length is only {} bytes.", self.needed_size(), buffer.len());
        }
        // total needed buffer size
        let mut range: Range<usize> = RangeIterator::new(USIZE_LEN);
        let total_needed_size = self.needed_size() as u32;
        total_needed_size.to_bytes(&mut buffer[range.clone()]).expect("Could not persist total_needed_size");
        // header_flags
        range.increment(HEADER_FLAGS_LEN);
        let headerflags_numeric = self.header_flags.bits();
        headerflags_numeric.to_bytes(&mut buffer[range.clone()]).expect("Could not persist header_flags");
        // uri
        serialize_vec_u8("IotaBridgeRequestParts", "uri_bytes", &self.uri_bytes, buffer, &mut range);
        // body_bytes
        serialize_vec_u8("IotaBridgeRequestParts", "body_bytes", &self.body_bytes, buffer, &mut range);
        Ok(range.end)
    }

    fn try_from_bytes(buffer: &[u8]) -> anyhow::Result<Self> where Self: Sized {
        // total needed buffer size
        let (buffer_length_is_correct, mut range, total_needed_size) = is_request_buffer_length_correct(buffer, buffer.len());
        if !buffer_length_is_correct {
            panic!("[BinaryPersist for IotaBridgeRequestParts - try_from_bytes()] This Request needs {} bytes but \
                    the provided buffer length is only {} bytes.", total_needed_size, buffer.len());
        }
        // header_flags
        range.increment(HEADER_FLAGS_LEN);
        let header_flags_numerical = HeaderFlagsNumericalType::try_from_bytes(
            &buffer[range.clone()]).expect("Error while deserializing numerical representation of header_flags");
        let header_flags = HeaderFlags::from_bits(header_flags_numerical).ok_or_else(
            || panic!("Error while interpreting header_flags_numerical as binary header_flags: Numerical value is {}", header_flags_numerical ))
            .expect("Error while unwrapping header_flags");
        // uri
        let uri = deserialize_string(buffer, &mut range)?;
        // body
        let body_bytes = deserialize_vec_u8("IotaBridgeRequestParts", "body_bytes", &buffer, &mut range);

        Ok(IotaBridgeRequestParts::new(header_flags, uri, body_bytes))
    }
}


#[derive(Debug, PartialEq)]
pub struct IotaBridgeResponseParts {
    pub body_bytes: Vec<u8>,
    pub status_code: StatusCode,
}

impl IotaBridgeResponseParts {
    pub fn new_for_closed_socket_connection() -> Self {
        IotaBridgeResponseParts {
            body_bytes: vec![],
            status_code: StatusCode::CONFLICT,
        }
    }

    pub fn is_closed_socket_connection(&self) -> bool {
        self.status_code != StatusCode::CONFLICT
    }

    pub async fn from_hyper_response(response: HyperResponse<Body>) -> Self {
        let status_code = response.status();
        let body_bytes = body::to_bytes(response.into_body()).await.unwrap();
        log::debug!("[from_hyper_response()] Returning IotaBridgeResponseParts with status {} and {} body bytes.",
                    status_code,
                    body_bytes.len());
        Self {
            body_bytes: Vec::<u8>::from(body_bytes),
            status_code,
        }
    }

    pub fn persist_to_hyper_response(self: Self, response_status: StatusCode) -> Result<HyperResponse<Body>> {
        let mut buffer: Vec<u8> = vec![0; self.needed_size()];
        self.to_bytes(buffer.as_mut_slice()).expect("Could not serialize IotaBridgeResponseParts into buffer");
        log::debug!("[persist_to_hyper_response_200()] Serialized this IotaBridgeResponseParts to binary data:\
        \n    length:{}\n    bytes:{:02X?}", buffer.len(), buffer.as_slice());
        HyperResponse::builder()
            .status(response_status)
            .body(Body::from(buffer))
    }
}

impl BinaryPersist for IotaBridgeResponseParts {
    fn needed_size(&self) -> usize {
        // status_code: u16 -> 2 byte
        // body_bytes: u32 size + bytes_len -> 4 byte + bytes_len
        2 + 4 + self.body_bytes.len()
    }

    fn to_bytes(&self, buffer: &mut [u8]) -> anyhow::Result<usize> {
        if buffer.len() < self.needed_size() {
            panic!("[BinaryPersist for IotaBridgeResponseParts - to_bytes()] This Response needs {} bytes but \
                    the provided buffer length is only {} bytes.", self.needed_size(), buffer.len());
        }
        // status_code
        let u16_status_code = self.status_code.as_u16();
        let mut range: Range<usize> = RangeIterator::new(<u16 as BinaryPersist>::needed_size(&u16_status_code));
        u16::to_bytes(&u16_status_code, &mut buffer[range.clone()]).expect("Could not persist u16_status_code");
        // body_bytes
        serialize_vec_u8("IotaBridgeResponseParts", "body_bytes", &self.body_bytes, buffer, &mut range);
        Ok(range.end)
    }

    fn try_from_bytes(buffer: &[u8]) -> anyhow::Result<Self> where Self: Sized {
        // status_code
        let u16_dummy = 0u16;
        let u16_size = <u16 as BinaryPersist>::needed_size(&u16_dummy);
        let mut range: Range<usize> = RangeIterator::new(u16_size);
        let status_code_u16 = u16::try_from_bytes(&buffer[range.clone()]).unwrap();
        // body_bytes
        let body_bytes = deserialize_vec_u8("IotaBridgeResponseParts", "body_bytes", &buffer, &mut range);
        Ok(Self {
            body_bytes,
            status_code: StatusCode::from_u16(status_code_u16)?,
        })
    }
}

impl fmt::Display for IotaBridgeResponseParts {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "IotaBridgeResponseParts: status: {}, body length: {}", self.status_code, self.body_bytes.len())
    }
}

pub mod streams_poc_lib_ffi {
    use std::os::raw::c_char;

    /// Used with post_binary_request_to_iota_bridge() function
    /// @param dev_eui              DevEUI of the sensor used by the IOTA-Bridge to identify the sensor.
    /// @param iota_bridge_url      URL of the iota-bridge instance to connect to.
    ///                                 Example: "http://192.168.0.100:50000"
    #[allow(non_camel_case_types)]
    #[repr(C)]
    pub struct iota_bridge_tcpip_proxy_options_t {
        pub dev_eui: *const c_char,
        pub iota_bridge_url: *const c_char,
    }
}

#[derive(Debug, Clone)]
pub struct IotaBridgeTcpIpProxySettings {
    pub dev_eui: String,
    pub iota_bridge_url: String,
}

impl IotaBridgeTcpIpProxySettings {
    pub fn new_from_iota_bridge_proxy_opt(iota_bridge_proxy_opt: *const streams_poc_lib_ffi::iota_bridge_tcpip_proxy_options_t) -> Option<Self> {
        if iota_bridge_proxy_opt == ptr::null() {
            return None
        }

        let dev_eui  = Self::convert_c_str_to_string(unsafe{(*iota_bridge_proxy_opt).dev_eui}, STREAMS_TOOLS_CONST_DEV_EUI_NOT_DEFINED);
        let iota_bridge_url = Self::convert_c_str_to_string(unsafe{(*iota_bridge_proxy_opt).iota_bridge_url}, "");

        Some(Self { dev_eui, iota_bridge_url })
    }

    fn convert_c_str_to_string(c_str_ptr: *const c_char, default_val: &str) -> String {
        let ret_val = if c_str_ptr != ptr::null() {
            let cstr_in: &CStr = unsafe { CStr::from_ptr(c_str_ptr) };
            match cstr_in.to_str() {
                Ok(utf8_str_in) => {
                    String::from(utf8_str_in)
                }
                Err(e) => format!("Utf8Error: {}", e)
            }
        } else {
            default_val.to_string()
        };
        ret_val
    }
}

impl BinaryPersist for IotaBridgeTcpIpProxySettings {
    fn needed_size(&self) -> usize {
        // dev_eui: u64 -> USIZE_LEN + string_data
        // iota_bridge_url: USIZE_LEN + string_data
        USIZE_LEN + self.dev_eui.len() + USIZE_LEN + self.iota_bridge_url.len()
    }

    fn to_bytes(&self, buffer: &mut [u8]) -> anyhow::Result<usize> {
        if buffer.len() < self.needed_size() {
            panic!("[BinaryPersist for IotaBridgeTcpIpProxySettings - to_bytes()] This struct needs {} bytes but \
                    the provided buffer length is only {} bytes.", self.needed_size(), buffer.len());
        }
        // dev_eui
        let mut range: Range<usize> = RangeIterator::new(0);
        serialize_string(&self.dev_eui, buffer, &mut range)?;
        // iota_bridge_url
        serialize_string(&self.iota_bridge_url, buffer, &mut range)?;

        Ok(range.end)
    }

    fn try_from_bytes(buffer: &[u8]) -> anyhow::Result<Self> where Self: Sized {
        let mut range: Range<usize> = RangeIterator::new(0);
        // dev_eui
        let dev_eui = deserialize_string(&buffer, &mut range)?;
        // iota_bridge_url
        let iota_bridge_url = deserialize_string(&buffer, &mut range)?;

        Ok(Self {dev_eui, iota_bridge_url})
    }
}