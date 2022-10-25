use hyper::{
    Body,
    body,
    http::{
        Result,
        StatusCode,
        Response as HyperResponse,
        request::{
            Builder,
            Request,
        }
    }
};

use std::{
    fmt,
    fmt::Formatter,
    ops::Range,
};
use crate::binary_persist::{
    BinaryPersist,
    USIZE_LEN,
    RangeIterator,
    deserialize_string
};
use std::{
    str::FromStr
};

use anyhow::bail;

#[derive(Debug)]
pub enum HttpMethod {
    POST,
    GET,
}

impl fmt::Display for HttpMethod {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            HttpMethod::GET => write!(f, "GET"),
            HttpMethod::POST => write!(f, "POST"),
        }
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

pub struct IotaBridgeRequestParts {
    pub method: HttpMethod,
    pub uri: String,
    pub body_bytes: Vec<u8>,
    uri_bytes: Vec<u8>,
    method_bytes: Vec<u8>,
}

impl IotaBridgeRequestParts {
    pub fn new(method: HttpMethod, uri: String, body_bytes: Vec<u8>) -> Self {
        let method_bytes = method.to_string().into_bytes();
        let uri_bytes = uri.clone().into_bytes();
        Self {method, uri, body_bytes, uri_bytes, method_bytes}
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
}

impl fmt::Display for IotaBridgeRequestParts {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "IotaBridgeRequestParts:
                     method: {}
                     uri: {}
                     body length: {}
                ", self.method, self.uri, self.body_bytes.len())
    }
}

fn serialize_vec_u8(struct_name: &str, prop_name: &str, bytes: &Vec<u8>, buffer: &mut [u8], range: &mut Range<usize>) {
    let bytes_len = bytes.len() as u32;
    range.increment(USIZE_LEN);
    u32::to_bytes(&bytes_len, &mut buffer[range.clone()]).expect(format!("Could not persist {} size", prop_name).as_str());
    log::debug!("[BinaryPersist for {} - to_bytes()] {} byte length: {}", struct_name, prop_name, bytes_len);
    if bytes_len > 0 {
        range.increment(bytes_len as usize);
        buffer[range.clone()].clone_from_slice(bytes.as_slice());
        log::debug!("[BinaryPersist for {} - to_bytes()] {}: {:02X?}", struct_name, prop_name, buffer[range.start..range.end].to_vec());
    } else {
        log::debug!("[BinaryPersist for {} - to_bytes()] {}: []", struct_name, prop_name);
    }
}

fn deserialize_vec_u8(struct_name: &str, prop_name: &str, buffer: &&[u8], range: &mut Range<usize>) -> Vec<u8>{
    range.increment(USIZE_LEN);
    let bytes_len = u32::try_from_bytes(&buffer[range.clone()]).unwrap();
    log::debug!("[BinaryPersist for {} - try_from_bytes] {}: {}", struct_name, prop_name, bytes_len);
    range.increment(bytes_len as usize);
    let ret_val: Vec<u8> = buffer[range.clone()].to_vec();
    log::debug!("[BinaryPersist for {} - try_from_bytes()] {}: {:02X?}", struct_name, prop_name, buffer[range.start..range.end].to_vec());
    ret_val
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
        // 1 - total needed buffer size
        // 1 - method
        // 2 - uri
        // 3 - body
        let length_values_size = 4 * USIZE_LEN;
        length_values_size + self.method_bytes.len() + self.uri_bytes.len() + self.body_bytes.len()
    }

    fn to_bytes(&self, buffer: &mut [u8]) -> anyhow::Result<usize> {
        if buffer.len() < self.needed_size() {
            panic!("[BinaryPersist for IotaBridgeRequestParts - to_bytes()] This Request needs {} bytes but \
                    the provided buffer length is only {} bytes.", self.needed_size(), buffer.len());
        }
        // total needed buffer size
        let mut range: Range<usize> = RangeIterator::new(USIZE_LEN);
        let total_needed_size = self.needed_size() as u32;
        u32::to_bytes(&total_needed_size, &mut buffer[range.clone()]).expect("Could not persist total_needed_size");
        // method
        serialize_vec_u8("IotaBridgeRequestParts", "method_bytes", &self.method_bytes, buffer, &mut range);
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
        // method
        let method_str = deserialize_string(buffer, &mut range)?;
        let method = HttpMethod::from_str(method_str.as_str())?;
        // uri
        let uri = deserialize_string(buffer, &mut range)?;
        // body
        let body_bytes = deserialize_vec_u8("IotaBridgeRequestParts", "body_bytes", &buffer, &mut range);

        Ok(IotaBridgeRequestParts::new(method, uri, body_bytes))
    }
}


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

    pub fn persist_to_hyper_response_200(self: Self) -> Result<HyperResponse<Body>> {
        let mut buffer: Vec<u8> = vec![0; self.needed_size()];
        self.to_bytes(buffer.as_mut_slice()).expect("Could not serialize IotaBridgeResponseParts into buffer");
        log::debug!("[persist_to_hyper_response_200()] Serialized this IotaBridgeResponseParts to binary data:\
        \n    length:{}\n    bytes:{:02X?}", buffer.len(), buffer.as_slice());
        HyperResponse::builder()
            .status(StatusCode::from_u16(200u16)?)
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
