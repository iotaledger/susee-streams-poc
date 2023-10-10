use std::{
    clone::Clone,
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
        Command,
    },
    http::{
        DispatchScope,
        ScopeConsume,
        http_protocol_command::{
            ServerDispatchCommand,
            URI_PREFIX_COMMAND,
        }
    },
};

use super::{
    fifo_queue::{
        FifoQueue,
        FifoQueueElement,
        fifo_queue_pop_front
    },
};

static mut FIFO_QUEUE: Option<FifoQueue> = None;

pub struct DispatchCommand<'a> {
    fifo: &'a mut FifoQueue,
    scope: Option<Rc<dyn DispatchScope>>,
}

impl<'a> Clone for DispatchCommand<'a> {
    fn clone(&self) -> DispatchCommand<'a> {
        let fifo_queue: & mut FifoQueue;
        unsafe {
            // TODO: This unsafe code needs to be replaced by a thread safe shared queue instance
            //       based on Arc::new(Mutex::new(......)) as been described here
            //       https://stackoverflow.com/questions/60996488/passing-additional-state-to-rust-hyperserviceservice-fn
            if FIFO_QUEUE.is_none() {
                FIFO_QUEUE = Some(FifoQueue::new());
            }
            fifo_queue = FIFO_QUEUE.as_mut().unwrap()
        }
        Self {
            fifo: fifo_queue,
            scope: self.scope.clone()
        }
    }
}

impl<'a> DispatchCommand<'a>
{
    pub fn new() -> Self {
        let fifo_queue: & mut FifoQueue;
        unsafe {
            // TODO: This unsafe code needs to be replaced by ... (See comment in the unsafe scope above)
            if FIFO_QUEUE.is_none() {
                FIFO_QUEUE = Some(FifoQueue::new());
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
impl<'a> ServerDispatchCommand for DispatchCommand<'a> {

    fn get_uri_prefix(&self) -> &'static str { URI_PREFIX_COMMAND }

    async fn fetch_next_command(self: &mut Self) -> Result<Response<Body>> {
        if let Some(req_body_binary) = fifo_queue_pop_front(self.fifo) {
            let cmd = Command::try_from_bytes(req_body_binary.payload.as_slice()).expect("Could not deserialize command from outgoing binary http body.");
            log::info!("[fn fetch_next_command()] Returning command {}.\nBlob length: {}\nQueue length: {}",
                    cmd,
                    req_body_binary.payload.len(),
                    self.fifo.len(),
            );
            Ok(Response::new(req_body_binary.payload.into()))
        } else {
            log::debug!("[fn fetch_next_command()] No command available");
            log::info!("[fn fetch_next_command()] Returning Command::NO_COMMAND");
            let mut buffer: [u8; Command::LENGTH_BYTES] = [0; Command::LENGTH_BYTES];
            Command::NO_COMMAND.to_bytes(&mut buffer).unwrap();
            Ok(Response::new(Body::from(buffer.to_vec())))
        }
    }

    async fn register_remote_command(self: &mut Self, req_body_binary: &[u8], api_fn_name: &str) -> Result<Response<Body>> {
        self.fifo.push_back(FifoQueueElement::from_binary(req_body_binary));
        let cmd = Command::try_from_bytes(req_body_binary).expect("Could not deserialize command from incoming binary http body.");
        log::info!("[fn {}()] Received command {}.\nBinary length: {}\nQueue length: {}",
                 api_fn_name,
                 cmd,
                 req_body_binary.len(),
                 self.fifo.len(),
        );
        Ok(Response::new(Default::default()))
    }
}

#[async_trait(?Send)]
impl<'a> ScopeConsume for DispatchCommand<'a> {
    fn set_scope(&mut self, scope: Rc<dyn DispatchScope>) {
        self.scope = Some(scope);
    }
}
