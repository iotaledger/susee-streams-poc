use hyper::{
    Body,
    http::{
        Response,
        StatusCode,
        Error,
        request::{
            Builder,
            Request,
        }
    }
};
use crate::BinaryPersist;
use crate::binary_persist::EnumeratedPersistable;

#[derive(Clone)]
pub struct RequestBuilderTools {
    pub uri_prefix: String,
}

impl RequestBuilderTools {
    pub fn new(uri_prefix: &str) -> Self {
        Self {
            uri_prefix: String::from(uri_prefix)
        }
    }

    pub fn get_request_builder(self: &Self) -> Builder {
        Request::builder().header("User-Agent", "streams-client/1.0")
    }

    pub fn get_uri(self: &Self, path: &str) -> String {
        format!("{}{}", self.uri_prefix, path)
    }

    pub fn send_enumerated_persistable_args<T: BinaryPersist>(&self, enumerated_persistable_args: T, path: &str) -> Result<Request<Body>,Error> {
        let mut buffer: Vec<u8> = vec![0; enumerated_persistable_args.needed_size()];
        enumerated_persistable_args.to_bytes(buffer.as_mut_slice()).expect("Persisting into binary data failed");

        self.get_request_builder()
            .method("POST")
            .uri(self.get_uri(path).as_str())
            .body(Body::from(buffer))
    }
}

pub fn get_response_404() -> Result<Response<Body>,Error> {
    let mut not_found = Response::default();
    *not_found.status_mut() = StatusCode::NOT_FOUND;
    Ok(not_found)
}

// Use the the persisted Command::XXXX_XXXX_XXXX instead as Response<Body>
pub fn get_body_bytes_from_enumerated_persistable<T: EnumeratedPersistable + BinaryPersist>(enumerated_persistable: &T) -> Result<[u8; T::LENGTH_BYTES],Error> {
    let mut buffer: [u8; T::LENGTH_BYTES] = [0; T::LENGTH_BYTES];
    enumerated_persistable.to_bytes(&mut buffer).unwrap();
    Ok(buffer)
}
