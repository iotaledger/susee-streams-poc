use std::{
    collections::VecDeque,
    cell::RefCell,
    rc::Rc,
};


use log;

use async_trait::async_trait;

use lets::{
    message::TransportMessage,
    transport::Transport,
    address::Address,
    error::Result as LetsResult,
};

use super::server_dispatch_streams::TransportFactory;

const MAX_POOL_SIZE: usize = 30;

pub struct TransportHandle<'a> {
    transport: Rc<RefCell<dyn Transport<'a, Msg=TransportMessage, SendResponse=TransportMessage>>>,
    instance_pos: usize,
}

#[async_trait(?Send)]
impl<'a> Transport<'a> for TransportHandle<'a>
{
    type Msg = TransportMessage;
    type SendResponse = TransportMessage;

    async fn send_message(&mut self, address: Address, msg: Self::Msg) -> LetsResult<Self::SendResponse> {
        self.transport.borrow_mut().send_message(address, msg).await
    }

    async fn recv_messages(&mut self, address: Address) -> LetsResult<Vec<Self::Msg>> {
        self.transport.borrow_mut().recv_messages(address).await
    }
}

#[async_trait(?Send)]
pub trait StreamsTransportPool {
    async fn get_transport(&mut self) -> Option<TransportHandle>;
    fn release_transport<'a>(&mut self, handle: &TransportHandle<'a>);
}

pub struct StreamsTransportPoolImpl<FactoryT: TransportFactory> {
    transport_factory: FactoryT,
    instances: Vec<Rc<RefCell<FactoryT::Output>>>,
    available: VecDeque<usize>,
}

impl<FactoryT: TransportFactory> StreamsTransportPoolImpl<FactoryT> {
    pub fn new(transport_factory: FactoryT) -> Self {
        StreamsTransportPoolImpl {
            transport_factory,
            instances: vec![],
            available: Default::default(),
        }
    }
}

#[async_trait(?Send)]
impl<FactoryT> StreamsTransportPool for StreamsTransportPoolImpl<FactoryT>
    where
        FactoryT: TransportFactory,
        for<'a> <FactoryT as TransportFactory>::Output: Transport<'a, Msg = TransportMessage, SendResponse = TransportMessage> + 'static
{

    async fn get_transport(&mut self) -> Option<TransportHandle> {
        let mut ret_val = None;

        while ret_val.is_none() {
            match self.available.pop_front() {
                Some(instance_pos) => {
                    log::debug!("available.pop_front() -> Some - available.len is {}", self.available.len());
                    ret_val = Some(TransportHandle{
                        transport: self.instances[instance_pos.clone()].clone(),
                        instance_pos
                    })
                }
                None => {
                    if self.instances.len()  < MAX_POOL_SIZE {
                        self.instances.push(self.transport_factory.new_transport().await);
                        let new_instance_pos = self.instances.len() - 1;
                        log::info!("Creating new transport with new_instance_pos {}", new_instance_pos);
                        self.available.push_back(new_instance_pos);
                        log::debug!("available.push_back({}) - available.len is {}", new_instance_pos, self.available.len());
                    } else {
                        log::warn!("MAX_POOL_SIZE of {} instances has been reached. No instance available. Try again later.", MAX_POOL_SIZE);
                        break;
                    }
                }
            }
        }

        ret_val
    }

    fn release_transport<'a>(&mut self, handle: &TransportHandle<'a>) {
        self.available.push_back(handle.instance_pos.clone());
        log::debug!("available.push_back({}) - available.len is {}", handle.instance_pos.clone(), self.available.len());
    }
}