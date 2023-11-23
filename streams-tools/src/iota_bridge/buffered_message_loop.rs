use std::{
    rc::Rc,
    time::Duration
};

use tokio::{
    time
};

use log;

use anyhow::{
    Result
};

use lets::{
    transport::{
        tangle::Client,
        Transport,
    },
    message::TransportMessage,
    error::{
        Result as LetsResult,
        Error as LetsError,
    },
};

use crate::{
    dao_helpers::{
        Condition,
        Conditions,
        Limit,
        MatchType
    },
    iota_bridge::{
        BufferedMessageDataStore,
        dao::BufferedMessage,
    },
    helpers::get_iota_node_url,
    user_manager::message_indexer::{
        MessageIndexer,
        MessageIndexerOptions
    },
};

type LetsClient = Client::<MessageIndexer, TransportMessage, TransportMessage>;

#[derive(Clone)]
pub struct BufferedMessageLoopOptions {
    pub buffered_message_store_factory: Rc<dyn Fn() -> BufferedMessageDataStore>,
    pub iota_node: String,
    pub send_messages_interval_in_secs: u64,
    pub max_send_messages_working_time_in_secs: u64,
}

impl BufferedMessageLoopOptions {
    pub fn new(iota_node: &str, buffered_msg_store_factory: impl Fn() -> BufferedMessageDataStore + 'static) -> BufferedMessageLoopOptions {
        BufferedMessageLoopOptions {
            buffered_message_store_factory: Rc::new(buffered_msg_store_factory),
            iota_node: iota_node.to_string(),
            send_messages_interval_in_secs: 5,
            max_send_messages_working_time_in_secs: 1,
        }
    }
}

#[derive(Debug)]
struct LoopStatus {
    pub remaining: usize,
    pub processed: usize,
    pub lets_err: Option<LetsError>,
}

impl LoopStatus {
    pub fn log_status(&self) {
        if self.lets_err.is_none() {
            log::info!("[fn run_buffered_message_loop] Finished sending all buffered messages(). {} messages processed. {} messages remaining.",
                self.processed,
                self.remaining
            );
        } else {
            log::error!("[fn run_buffered_message_loop] Got LetsError '{}'. {} messages processed. {} messages remaining.",
                self.lets_err.as_ref().unwrap(),
                self.processed,
                self.remaining,
            );
        }
    }

    pub fn should_break_loop(&self) -> bool {
        self.remaining == 0 || self.lets_err.is_some()
    }
}

pub async fn run_buffered_message_loop(opt: BufferedMessageLoopOptions) {
    let mut interval = time::interval(time::Duration::from_secs(opt.send_messages_interval_in_secs));
    loop {
        interval.tick().await;
        log::debug!("[fn run_buffered_message_loop] {} Seconds passed - Starting send_all_buffered_messages()", opt.send_messages_interval_in_secs);
        match send_all_buffered_messages(opt.clone()).await {
            Ok(loop_status) => {
                loop_status.log_status();
            }
            Err(err) => {
                log::error!("[fn run_buffered_message_loop] Got error from send_all_buffered_messages(): {}", err);
            }
        }
    }
}

async fn send_all_buffered_messages(opt: BufferedMessageLoopOptions) -> Result<LoopStatus>{
    let loop_start = time::Instant::now();
    let max_duration_to_run_loop = Duration::from_secs(opt.max_send_messages_working_time_in_secs);
    let mut buffered_message_store = (opt.buffered_message_store_factory)();
    log::debug!("[fn send_all_buffered_messages] Starting iterate_messages loop");
    let mut ret_val = LoopStatus {
        remaining: 0,
        processed: 0,
        lets_err: None,
    };
    'iterate_messages: loop {
        log::debug!("[fn send_all_buffered_messages] Calling check_buffered_message_existence_and_handle_it()");
        match check_buffered_message_existence_and_handle_it(opt.iota_node.as_str(), &mut buffered_message_store).await {
            Ok(loop_status) => {
                log::debug!("[fn send_all_buffered_messages] check_buffered_message_existence_and_handle_it returned loop_status: {:?}", loop_status);
                ret_val.processed += loop_status.processed;
                ret_val.remaining = loop_status.remaining;
                if loop_status.should_break_loop() {
                    log::debug!("[fn send_all_buffered_messages] Breaking iterate_messages loop due to loop_status.should_break_loop()");
                    ret_val.lets_err = loop_status.lets_err;
                    break 'iterate_messages;
                }
            }
            Err(err) => {
                log::error!("[fn send_all_buffered_messages] Breaking iterate_messages loop due to Error: {}", err);
                break 'iterate_messages;
            }
        }
        let duration_since_loop_start = time::Instant::now().duration_since(loop_start);
        log::debug!("[fn send_all_buffered_messages] duration_since_loop_start: {} millis", duration_since_loop_start.as_millis());
        if duration_since_loop_start >= max_duration_to_run_loop {
            log::debug!("[fn send_all_buffered_messages] Breaking iterate_messages loop due to max_duration_to_run_loop exceeded");
            break 'iterate_messages;
        }
    };
    Ok(ret_val)
}

async fn check_buffered_message_existence_and_handle_it(iota_node: &str, buffered_message_store: &mut BufferedMessageDataStore) -> Result<LoopStatus> {
    let mut conditions = Vec::<Condition>::new();
    let mut conditions_mngr = Conditions(&mut conditions);
    conditions_mngr.add(None, "link", MatchType::ListEverything);
    let limit = Limit{ limit: 1, offset: 0 };
    let (messages, total_cnt) = buffered_message_store.filter(conditions, Some(limit))?;
    log::debug!("[fn check_buffered_message_existence_and_handle_it] messages.len = {}, total_cnt: {}", messages.len(), total_cnt);
    let ret_val = if messages.len() > 0 {
        match handle_buffered_messages(iota_node, messages, buffered_message_store).await {
            Ok(proc_msgs) => LoopStatus{ remaining: total_cnt - proc_msgs, processed: proc_msgs, lets_err: None },
            Err(err) => {
                LoopStatus{ remaining: total_cnt, processed: 0, lets_err: Some(err)}
            },
        }
    } else {
        log::debug!("[fn check_buffered_message_existence_and_handle_it] Number of buffered_messages in store is 0");
        if total_cnt > 0 {
            log::error!("[fn check_buffered_message_existence_and_handle_it] Buffered_messages list from store is empty although total_cnt in store is: {}", total_cnt);
        }
        LoopStatus{ remaining: total_cnt, processed: 0, lets_err: None }
    };
    Ok(ret_val)
}

async fn handle_buffered_messages(
    iota_node: &str,
    messages: Vec::<BufferedMessage>,
    buffered_message_store: &mut BufferedMessageDataStore
) -> LetsResult<usize> {
    let mut client = create_lets_client(iota_node).await?;
    let mut processed_messages: usize = 0;
    for message in messages {
        processed_messages += send_buffered_message(&mut client, &message, buffered_message_store).await?;
    }
    Ok(processed_messages)
}

async fn create_lets_client(iota_node: &str) -> LetsResult<LetsClient> {
    let indexer = MessageIndexer::new(MessageIndexerOptions::new(iota_node.to_string()));
    LetsClient::for_node(
            &get_iota_node_url(iota_node),
            indexer
        )
        .await
}

async fn send_buffered_message(client: &mut LetsClient, message: &BufferedMessage, buffered_message_store: &mut BufferedMessageDataStore) -> LetsResult<usize> {
    let _response = client.send_message(
            message.link.parse().unwrap(),
            TransportMessage::new(message.body.clone())
        )
        .await?
    ;
    log::debug!("[fn send_buffered_message] Successfully send message {} with id {}", message.link, message.id.unwrap_or(-1));
    let mut ret_val = 0;
    if let Some(id) = message.id.as_ref() {
        match buffered_message_store.delete_item_in_db(id) {
            Ok(_) => {
                log::info!("[fn send_buffered_message] Successfully processed buffered_message: {}", message.link);
                ret_val = 1;
            }
            Err(_) => {
                log::error!("[fn send_buffered_message] Could not delete buffered_message {}. Will succeed loop anyway.", id);
            }
        }
    } else {
        log::warn!("[fn send_buffered_message] Provided message '{}' has not been read from database. No delete required", message.link);
    }
    Ok(ret_val)
}

