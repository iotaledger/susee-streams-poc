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
        FifoQueueMap,
        FifoQueueElement,
        fifo_queue_pop_front,
        get_new_fifo_queue_map,
        create_new_fifo_queue_if_not_exist,
    },
};

static mut FIFO_QUEUES: Option<FifoQueueMap> = None;

pub struct DispatchCommand<'a> {
    fifos: &'a mut FifoQueueMap,
    scope: Option<Rc<dyn DispatchScope>>,
}

impl<'a> Clone for DispatchCommand<'a> {
    fn clone(&self) -> DispatchCommand<'a> {
        Self {
            fifos: Self::get_fifo_queues_ref_mut(),
            scope: self.scope.clone()
        }
    }
}

impl<'a> DispatchCommand<'a>
{
    fn get_fifo_queues_ref_mut() -> &'a mut FifoQueueMap {
        unsafe {
            // TODO: This unsafe code needs to be replaced by a thread safe shared queue instance
            //       based on Arc::new(Mutex::new(......)) as been described here
            //       https://stackoverflow.com/questions/60996488/passing-additional-state-to-rust-hyperserviceservice-fn
            if FIFO_QUEUES.is_none() {
                FIFO_QUEUES = Some(get_new_fifo_queue_map());
            }
            FIFO_QUEUES.as_mut().unwrap()
        }
    }

    pub fn new() -> Self {
        Self {
            fifos: Self::get_fifo_queues_ref_mut(),
            scope: None,
        }
    }

    fn get_no_command_response() -> Result<Response<Body>> {
        let mut buffer: [u8; Command::LENGTH_BYTES] = [0; Command::LENGTH_BYTES];
        Command::NO_COMMAND.to_bytes(&mut buffer).unwrap();
        Ok(Response::new(Body::from(buffer.to_vec())))
    }
}

#[async_trait(?Send)]
impl<'a> ServerDispatchCommand for DispatchCommand<'a> {

    fn get_uri_prefix(&self) -> &'static str { URI_PREFIX_COMMAND }

    async fn fetch_next_command(self: &mut Self, dev_eui: &str) -> Result<Response<Body>> {
        if let Some(fifo) = self.fifos.get_mut(dev_eui).as_deref_mut() {
            log::debug!("[fn fetch_next_command()] Found FifoQueue with length {} for dev_eui {}",
                fifo.len(), dev_eui
            );
            if let Some(req_body_binary) = fifo_queue_pop_front(fifo ) {
                let cmd = Command::try_from_bytes(req_body_binary.payload.as_slice()).expect("Could not deserialize command from outgoing binary http body.");
                log::info!("[fn fetch_next_command()] Returning command {}.\nBlob length: {}\nQueue length: {}",
                    cmd,
                    req_body_binary.payload.len(),
                    fifo.len(),
                );
                Ok(Response::new(req_body_binary.payload.into()))
            } else {
                log::debug!("[fn fetch_next_command()] No command available");
                log::info!("[fn fetch_next_command()] Returning Command::NO_COMMAND");
                Self::get_no_command_response()
            }
        } else {
            log::info!("[fn fetch_next_command()] DevEUI not known - Returning Command::NO_COMMAND");
            Self::get_no_command_response()
        }
    }

    async fn register_remote_command(self: &mut Self, dev_eui: &str, req_body_binary: &[u8], api_fn_name: &str) -> Result<Response<Body>> {
        create_new_fifo_queue_if_not_exist(&self.fifos, dev_eui);
        if let Some(fifo) = self.fifos.get_mut(dev_eui).as_deref_mut() {
            let cmd = Command::try_from_bytes(req_body_binary).expect("Could not deserialize command from incoming binary http body.");
            fifo.push_back( FifoQueueElement::from_binary(
                req_body_binary, cmd.needs_to_wait_for_tangle_milestone()
            ));
            log::info!("[fn {}()] Received command {}.\nBinary length: {}\nQueue length: {}",
                 api_fn_name,
                 cmd,
                 req_body_binary.len(),
                 fifo.len(),
            );
            Ok(Response::new(Default::default()))
        } else {
            log::error!("[fn register_remote_command()] Could not create FiFoQueue for DevEUI '{}' - Returning error 500", dev_eui);
            Ok(Response::builder()
                .status(500)
                .body(Default::default())
                .unwrap())
        }
    }
}

#[async_trait(?Send)]
impl<'a> ScopeConsume for DispatchCommand<'a> {
    fn set_scope(&mut self, scope: Rc<dyn DispatchScope>) {
        self.scope = Some(scope);
    }
}
