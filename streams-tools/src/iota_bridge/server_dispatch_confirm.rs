use std::{
    clone::Clone,
    collections::VecDeque,
    rc::Rc,
};

use async_trait::async_trait;

use hyper::{
    Body,
    http::{
        Response,
        Result,
    }
};

use crate::{
    binary_persist::{
        BinaryPersist,
        EnumeratedPersistable,
        Confirmation,
    },
    http::{
        ScopeConsume,
        DispatchScope,
        http_protocol_confirm::{
            ServerDispatchConfirm,
            URI_PREFIX_CONFIRM,
        }
    },
};

static mut FIFO_QUEUE: Option<VecDeque<Vec<u8>>> = None;

pub struct DispatchConfirm<'a> {
    fifo: &'a mut VecDeque<Vec<u8>>,
    scope: Option<Rc<dyn DispatchScope>>,
}

impl<'a> Clone for DispatchConfirm<'a> {
    fn clone(&self) -> DispatchConfirm<'a> {
        let fifo_queue: & mut VecDeque::<Vec<u8>>;
        unsafe {
            // TODO: This unsafe code needs to be replaced by a thread safe shared queue instance
            //       based on Arc::new(Mutex::new(......)) as been described here
            //       https://stackoverflow.com/questions/60996488/passing-additional-state-to-rust-hyperserviceservice-fn
            if FIFO_QUEUE.is_none() {
                FIFO_QUEUE = Some(VecDeque::<Vec<u8>>::new());
            }
            fifo_queue = FIFO_QUEUE.as_mut().unwrap()
        }
        Self {
            fifo: fifo_queue,
            scope: self.scope.clone(),
        }
    }
}

impl<'a> DispatchConfirm<'a>
{
    pub fn new() -> Self {
        let fifo_queue: & mut VecDeque::<Vec<u8>>;
        unsafe {
            // TODO: This unsafe code needs to be replaced by ... (See comment in the unsafe scope above)
            if FIFO_QUEUE.is_none() {
                FIFO_QUEUE = Some(VecDeque::<Vec<u8>>::new());
            }
            fifo_queue = FIFO_QUEUE.as_mut().unwrap()
        }

        Self {
            fifo: fifo_queue,
            scope: None,
        }
    }
}

#[async_trait(?Send)]
impl<'a> ServerDispatchConfirm for DispatchConfirm<'a> {

    fn get_uri_prefix(&self) -> &'static str { URI_PREFIX_CONFIRM }

    async fn fetch_next_confirmation(self: &mut Self) -> Result<Response<Body>> {
        if let Some(req_body_binary) = self.fifo.pop_front() {
            let confirm = Confirmation::try_from_bytes(req_body_binary.as_slice()).expect("Could not deserialize confirmation from outgoing binary http body.");
            println!("[IOTA-Bridge - DispatchConfirm] fetch_next_confirmation() - Returning confirmation {}.\nBlob length: {}\nQueue length: {}",
                    confirm,
                    req_body_binary.len(),
                    self.fifo.len(),
            );
            Ok(Response::new(req_body_binary.into()))
        } else {
            println!("[IOTA-Bridge - DispatchConfirm] fetch_next_confirmation() - No confirmation available. Returning Confirmation::NO_CONFIRMATION.\n");
            let mut buffer: [u8; Confirmation::LENGTH_BYTES] = [0; Confirmation::LENGTH_BYTES];
            Confirmation::NO_CONFIRMATION.to_bytes(&mut buffer).unwrap();
            Ok(Response::new(Body::from(buffer.to_vec())))
        }
    }

    async fn register_confirmation(self: &mut Self, req_body_binary: &[u8], api_fn_name: &str) -> Result<Response<Body>> {
        self.fifo.push_back(req_body_binary.to_vec());
        let confirm = Confirmation::try_from_bytes(req_body_binary).expect("Could not deserialize confirmation from incoming binary http body.");
        println!("[IOTA-Bridge - DispatchConfirm] {}() - Received confirmation {}.\nBinary length: {}\nQueue length: {}",
                 api_fn_name,
                 confirm,
                 req_body_binary.len(),
                 self.fifo.len(),
        );
        Ok(Response::new(Default::default()))
    }
}

#[async_trait(?Send)]
impl<'a> ScopeConsume for DispatchConfirm<'a> {
    fn set_scope(&mut self, scope: Rc<dyn DispatchScope>) {
        self.scope = Some(scope);
    }
}