use std::{
    time::Duration
};

use anyhow::{
    Result,
    Error as AnyError
};

use async_trait::async_trait;

use tokio::{
    time::{
        Instant,
        interval,
    }
};

use lets::transport::tangle::Client;

use crate::{
    UserDataStore,
    MessageManager,
    user_manager::{
        dao::{
            User,
            message::MessageDataStoreOptions
        },
        message_indexer::MessageIndexer,
        multi_channel_management::{
            MultiChannelManagerOptions,
            get_channel_manager_for_channel_id,
        },
    },
    threading_helpers::Worker,
};

#[derive(Clone)]
pub struct SyncChannelsLoopOptions {
    pub user_store: UserDataStore,
    pub multi_channel_mngr_opt: MultiChannelManagerOptions,
    pub message_data_store_file_path_and_name: String,
    pub sync_channels_interval_in_secs: u64,
    pub sync_channels_stop_before_next_run_secs: u64,
}

unsafe impl Send for SyncChannelsLoopOptions {}
unsafe impl Sync for SyncChannelsLoopOptions {}

impl SyncChannelsLoopOptions {
    pub fn new(user_store: UserDataStore, multi_channel_mngr_opt: MultiChannelManagerOptions, msg_data_store_file_path_name: String) -> SyncChannelsLoopOptions {
        SyncChannelsLoopOptions {
            user_store,
            multi_channel_mngr_opt,
            message_data_store_file_path_and_name: msg_data_store_file_path_name,
            sync_channels_interval_in_secs: 3600,
            sync_channels_stop_before_next_run_secs: 600,
        }
    }
}

pub struct SyncChannelsWorker;

#[async_trait(?Send)]
impl Worker for SyncChannelsWorker {
    type OptionsType = SyncChannelsLoopOptions;
    type ResultType = ();
    type ErrorType = AnyError;

    async fn run(opt: SyncChannelsLoopOptions) -> Result<()> {
        run_sync_channels_loop(opt).await;
        Ok(())
    }
}

#[derive(Debug)]
struct UserLoopStatus {
    pub remaining: usize,
    pub processed: usize,
}

impl UserLoopStatus {
    fn new(remaining: usize, processed: usize) -> Self {
        UserLoopStatus { remaining, processed }
    }

    pub fn log_status(&self) {
        log::info!("[fn run_sync_channels_loop] Finished syncing channels(). {} channels processed. {} channels remaining.",
                   self.processed,
                   self.remaining
        );
    }
}

pub async fn run_sync_channels_loop(opt: SyncChannelsLoopOptions) {
    let mut interval = interval(Duration::from_secs(opt.sync_channels_interval_in_secs));
    loop {
        interval.tick().await;
        log::debug!("[fn run_sync_channels_loop] {} Seconds passed - Starting sync_all_channels()", opt.sync_channels_interval_in_secs);
        match sync_each_channel_in_user_store(opt.clone()).await {
            Ok(loop_status) => {
                loop_status.log_status();
            }
            Err(err) => {
                log::error!("[fn run_sync_channels_loop] Got error from sync_all_channels(): {}", err);
            }
        }
    }
}

async fn sync_each_channel_in_user_store(opt: SyncChannelsLoopOptions) -> Result<UserLoopStatus> {
    let loop_start = Instant::now();
    let max_duration_to_run_loop = Duration::from_secs(
        opt.sync_channels_interval_in_secs - opt.sync_channels_stop_before_next_run_secs
    );
    log::info!("[fn sync_each_channel_in_user_store] Start syncing channels of all users");
    let (mut users, items_cnt_total) = opt.user_store.find_all("", None)?;
    let mut ret_val = UserLoopStatus::new(items_cnt_total, 0);
    for user in users.iter_mut() {
        log::info!("[fn sync_each_channel_in_user_store] Starting syncing channel {}", user.streams_channel_id);
        match sync_channel(user, &opt).await {
            Ok(num_channels_processed) => {
                ret_val.processed += num_channels_processed;
                ret_val.remaining -= num_channels_processed;
            },
            Err(e) => {
                log::error!("[fn sync_each_channel_in_user_store] fn sync_channel returned error. Continuing loop over users. Error: {}", e);
            }
        };
        let duration_since_loop_start = Instant::now().duration_since(loop_start);
        log::debug!("[fn sync_each_channel_in_user_store] duration_since_loop_start: {} secs", duration_since_loop_start.as_secs());
        if duration_since_loop_start >= max_duration_to_run_loop {
            log::info!("[fn sync_each_channel_in_user_store] Breaking loop over users due to max duration exceeded {:?}", max_duration_to_run_loop);
            break;
        }
    }
    log::info!("[fn sync_each_channel_in_user_store] Stopped syncing channels of all users");
    Ok(ret_val)
}

async fn sync_channel(user: &mut User, opt: &SyncChannelsLoopOptions) -> Result<usize>{
    let mut multi_channel_mngr_opt = opt.multi_channel_mngr_opt.clone();
    multi_channel_mngr_opt.message_data_store_for_msg_caching = Some(
        MessageDataStoreOptions{
            file_path_and_name: opt.message_data_store_file_path_and_name.clone(),
            streams_channel_id: user.streams_channel_id.clone(),
        }
    );
    let mut channel_manager = get_channel_manager_for_channel_id(
        &user.streams_channel_id,
        &opt.user_store,
        &multi_channel_mngr_opt,
    ).await?;
    let mut num_channels_processed = 0;
    if let Some(author) = channel_manager.user.as_mut() {
        let mut msg_mngr = MessageManager::<Client<MessageIndexer>>::new(
            author,
            user.streams_channel_id.clone(),
            opt.message_data_store_file_path_and_name.clone()
        );
        msg_mngr.sync().await?;
        num_channels_processed = 1;
    }
    Ok(num_channels_processed)
}



