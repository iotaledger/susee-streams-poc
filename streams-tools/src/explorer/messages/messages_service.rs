use std::str::FromStr;

use async_trait::async_trait;

use hyper::http::StatusCode;

use streams::{
    Address,
};

use lets::transport::tangle::Client;

use crate::{
    threading_helpers::{
        Worker,
        run_worker_in_own_thread,
    },
    explorer::{
        app_state::{
            AppState,
            MessagesState,
        },
        error::{
            Result,
            AppError,
        },
        shared::PagingOptions,
    },
    user_manager::{
        dao::message::MessageDataStoreOptions,
        message_indexer::MessageIndexer,
        multi_channel_management::{
            MultiChannelManagerOptions,
            get_channel_manager_for_channel_id
        },
        MessageManager,
        UserDataStore,
    },
    dao_helpers::Limit,
    helpers::get_tangle_address_from_strings,
};

use super::{
    messages_dto::{
        Message,
        MessageList,
    },
};

impl MessagesState {
    pub(crate) fn as_multi_channel_manager_options(&self) -> MultiChannelManagerOptions {
        MultiChannelManagerOptions{
            iota_node: self.iota_node_url.clone(),
            wallet_filename: self.wallet_filename.clone(),
            streams_user_serialization_password: self.streams_user_serialization_password.clone(),
            message_data_store_for_msg_caching: None
        }
    }
}

pub(crate) async fn index(state: &AppState, channel_id: &str, paging_opt: Option<PagingOptions>) -> Result<(MessageList, usize)> {
    run_worker_in_own_thread::<IndexWorker>(IndexWorkerOptions::new(
        &state.messages,
        &state.user_store,
        channel_id,
        paging_opt,
    )).await
}

#[derive(Clone)]
struct IndexWorkerOptions {
    channel_id: String,
    u_store: UserDataStore,
    multi_channel_mngr_opt: MultiChannelManagerOptions,
    db_file_name: String,
    paging_opt: Option<PagingOptions>,
}

impl IndexWorkerOptions {
    pub fn new(messages: &MessagesState, user_store: &UserDataStore, channel_id: &str, paging_opt: Option<PagingOptions>) -> IndexWorkerOptions {
        let mut multi_channel_mngr_opt = messages.as_multi_channel_manager_options();
        multi_channel_mngr_opt.message_data_store_for_msg_caching = Some(MessageDataStoreOptions {
            file_path_and_name: messages.db_file_name.clone(),
            streams_channel_id: channel_id.to_string()
        });
        IndexWorkerOptions{
            channel_id: channel_id.to_string(),
            u_store: user_store.clone(),
            multi_channel_mngr_opt,
            db_file_name: messages.db_file_name.clone(),
            paging_opt,
        }
    }
}

struct IndexWorker;

#[async_trait(?Send)]
impl Worker for IndexWorker {
    type OptionsType = IndexWorkerOptions;
    type ResultType = (MessageList, usize);
    type ErrorType = AppError;

    async fn run(opt: IndexWorkerOptions) -> Result<(MessageList, usize)> {
        let mut channel_manager = match get_channel_manager_for_channel_id(
                &opt.channel_id,
                &opt.u_store,
                &opt.multi_channel_mngr_opt,
            ).await {
            Ok(mngr) => mngr,
            Err(_) => {
                return Err(AppError::ChannelDoesNotExist(opt.channel_id))
            }
        };
        if let Some(user) = channel_manager.user.as_mut() {
            let mut msg_mngr = MessageManager::<Client<MessageIndexer>>::new(
                user,
                opt.channel_id.clone(),
                opt.db_file_name
            );
            msg_mngr.sync().await?;
            let db_limit_offset = opt.paging_opt.map(|paging_opt| Limit::from(paging_opt));
            if let Ok((msg_meta_data_list, items_count_total)) = msg_mngr.index(db_limit_offset) {
                let mut ret_val = MessageList::new();
                for msg_meta_data in msg_meta_data_list {
                    let address = get_tangle_address_from_strings(opt.channel_id.as_str(), msg_meta_data.message_id.as_str())
                        .expect("get_tangle_address_from_strings error");
                    if let Ok(lets_msg) = user.receive_message(address).await{
                        ret_val.push(lets_msg.into());
                    } else {
                        ret_val.push(Message::new_from_id(
                            address.to_string(),
                            "Error could not receive message from tangle".to_string(),
                            "".to_string()
                        ).expect(format!("Error on creating Message::new_from_id with id {}", opt.channel_id).as_str()));
                    }
                }
                Ok((ret_val, items_count_total))
            } else {
                Err(AppError::InternalServerError(
                    format!("Could not get a msg_meta_data_list for channel {} from msg_mngr", opt.channel_id)
                ))
            }
        } else {
            Err(AppError::ChannelDoesNotExist(opt.channel_id))
        }
    }
}

pub(crate) async fn get(messages: &MessagesState, user_store: &UserDataStore, message_id: &str) -> Result<Message> {
    if let Ok(tangle_address) = Address::from_str(message_id) {
        run_worker_in_own_thread::<GetWorker>(GetWorkerOptions::new(
            tangle_address,
            messages,
            user_store,
        )).await
    } else {
        Err(AppError::GenericWithMessage(
            StatusCode::BAD_REQUEST,
            format!("Could not parse the channel-id from message-id {}. The message-id seems to be malformed.", message_id)
        ))
    }
}

#[derive(Clone)]
struct GetWorkerOptions {
    tangle_address: Address,
    u_store: UserDataStore,
    multi_channel_mngr_opt: MultiChannelManagerOptions,
}

impl GetWorkerOptions {
    pub fn new(tangle_address: Address, messages: &MessagesState, user_store: &UserDataStore) -> GetWorkerOptions {
        let mut multi_channel_mngr_opt = messages.as_multi_channel_manager_options();
        multi_channel_mngr_opt.message_data_store_for_msg_caching = Some(MessageDataStoreOptions {
            file_path_and_name: messages.db_file_name.clone(),
            streams_channel_id: tangle_address.base().to_string(),
        });
        GetWorkerOptions{
            tangle_address,
            u_store: user_store.clone(),
            multi_channel_mngr_opt,
        }
    }
}

struct GetWorker;

#[async_trait(?Send)]
impl Worker for GetWorker {
    type OptionsType = GetWorkerOptions;
    type ResultType = Message;
    type ErrorType = AppError;

    async fn run(opt: GetWorkerOptions) -> Result<Message> {
        let mut channel_manager = get_channel_manager_for_channel_id(
            &opt.tangle_address.base().to_string(),
            &opt.u_store,
            &opt.multi_channel_mngr_opt,
        ).await?;
        if let Some(author) = channel_manager.user.as_mut() {
            if let Ok(unwrapped_msg) = author.receive_message(opt.tangle_address).await {
                Ok(unwrapped_msg.into())
            } else {
                Err(AppError::GenericWithMessage(
                    StatusCode::NOT_FOUND,
                    format!("Could not receive message {} from tangle", opt.tangle_address.to_string())
                ))
            }
        } else {
            Err(AppError::ChannelDoesNotExist(opt.tangle_address.base().to_string()))
        }
    }
}