use super::{
    command_fetcher::{
        CommandFetcher,
        CommandFetcherOptions,
    },
    esp32_subscriber_tools::{
        create_subscriber,
        SubscriberManagerPlainTextWalletHttpClientEspRs,
    },
    http_client_smol_esp_rs::{
        HttpClient,
        HttpClientOptions,
    },
};

use crate::esp_rs::streams_poc_lib::api_types::send_request_via_lorawan_t;

use payloads::{
    Message,
    get_message_bytes,
};

use super::{
    wifi_utils::init_wifi,
};

use susee_tools::SUSEE_CONST_COMMAND_CONFIRM_FETCH_WAIT_SEC;

use streams_tools::{
    subscriber_manager::get_public_key_str,
    binary_persist::{
        Subscription,
        SubscriberStatus,
        Command,
    },
    PlainTextWallet,
    http::http_protocol_confirm::RequestBuilderConfirm,
    remote::command_processor::{
        CommandProcessor,
        SensorFunctions,
        CommandFetchLoopOptions,
        run_command_fetch_loop,
        process_sensor_commands,
    }
};

use iota_streams::{
    core::async_trait,
    app_channels::api::{
        tangle::{
            Address,
            Bytes,
            Subscriber,
        },
    }
};

use core::str::FromStr;

use anyhow::{
    anyhow,
    Result,
};

use hyper::{
    Body,
    http::{
        Request,
        status,
    }
};

type ClientType = HttpClient;

fn print_heap_info() {
    unsafe {
        let free_mem = esp_idf_sys::heap_caps_get_free_size(
            esp_idf_sys::MALLOC_CAP_8BIT
        );

        log::info!("[fn print_heap_info] heap_caps_get_free_size(MALLOC_CAP_8BIT): {}", free_mem);
    }
}

fn println_subscription_details(subscriber: &Subscriber<ClientType>, subscription_link: &Address, comment: &str, key_name: &str) {
    let public_key = get_public_key_str(subscriber);
    println!(
        "[Sensor] {}:
         {} Link:     {}
              Tangle Index:     {:#}
         Subscriber public key: {}\n",
        comment,
        key_name,
        subscription_link.to_string(),
        subscription_link.to_msg_index(),
        public_key,
    );
}

fn println_subscriber_status<'a> (
    subscriber_manager: &SubscriberManagerPlainTextWalletHttpClientEspRs,
    confirm_req_builder: &RequestBuilderConfirm
) -> hyper::http::Result<Request<Body>>
{
    let mut ret_val: Option<Request<Body>> = None;
    if let Some(subscriber) = &subscriber_manager.subscriber {
        if let Some(subscription_link) = subscriber_manager.subscription_link {
            println_subscription_details(&subscriber, &subscription_link, "A subscription with the following details has already been created", "Subscription");

            let mut previous_message_link= String::from("");
            if let Some(prev_msg_link) = subscriber_manager.prev_msg_link {
                previous_message_link = prev_msg_link.to_string();
            }

            ret_val = Some(
                confirm_req_builder.subscriber_status(
                    previous_message_link,
                    Subscription {
                        subscription_link: subscription_link.to_string(),
                        pup_key: get_public_key_str(subscriber),
                    })?
            );
        }
    }
    if ret_val.is_none() {
        println!("[Sensor] No subscription found.");
        let to_send = SubscriberStatus::default();
        ret_val = Some( confirm_req_builder.subscriber_status(to_send.previous_message_link, to_send.subscription)?);
    }

    if let Some(prev_msg_link) = subscriber_manager.prev_msg_link {
        println!(
            "[Sensor] Previous message:
         Prev msg link:     {}
             Tangle Index:     {:#}",
            prev_msg_link.to_string(),
            prev_msg_link.to_msg_index()
        );
    }

    // hyper::http::Error::from(InvalidStatusCode::from(404).unwrap())
    ret_val.ok_or_else(|| {
        if let Err(e) = status::StatusCode::from_u16(404) {
            e.into()
        } else {
            panic!("Should never happen");
        }
    })
}

async fn clear_client_state<'a> (
    subscriber_manager: &mut SubscriberManagerPlainTextWalletHttpClientEspRs,
    confirm_req_builder: &RequestBuilderConfirm
) -> hyper::http::Result<Request<Body>>
{
    subscriber_manager.clear_client_state().await.expect("subscriber_manager.clear_client_state() returned error");
    confirm_req_builder.clear_client_state()
}

pub async fn send_content_as_msg(
    message_key: String,
    subscriber: &mut SubscriberManagerPlainTextWalletHttpClientEspRs,
    confirm_req_builder: &RequestBuilderConfirm
) -> hyper::http::Result<Request<Body>>
{
    let message_bytes = get_message_bytes(Message::from(message_key.as_str()));
    log::info!("[fn send_content_as_msg] Sending {} bytes payload\n", message_bytes.len());
    log::debug!("[fn send_content_as_msg] - send_content_as_msg()] Message text: {}", std::str::from_utf8(message_bytes).expect("Could not deserialize message bytes to utf8 str"));
    let prev_message = subscriber.send_signed_packet(&Bytes(message_bytes.to_vec())).await.expect("subscriber.send_signed_packet() returned error");
    confirm_req_builder.send_message(prev_message.to_string())
}

async fn subscribe_to_channel(
    announcement_link_str: &str,
    subscriber_mngr: &mut SubscriberManagerPlainTextWalletHttpClientEspRs,
    confirm_req_builder: &RequestBuilderConfirm
) -> hyper::http::Result<Request<Body>>
{
    let ann_address = Address::from_str(&announcement_link_str).expect("Address::from_str() returned error");
    let sub_msg_link = subscriber_mngr.subscribe(&ann_address).await.expect("subscriber_mngr::subscribe() returned error");

    let subscriber = subscriber_mngr.subscriber.as_ref().unwrap();

    println_subscription_details(
        &subscriber,
        &sub_msg_link,
        "New subscription",
        "Subscription",
    );

    confirm_req_builder.subscription(sub_msg_link.to_string(), get_public_key_str(subscriber))
}

async fn register_keyload_msg(
    keyload_msg_link_str: &str,
    subscriber_mngr: &mut SubscriberManagerPlainTextWalletHttpClientEspRs,
    confirm_req_builder: &RequestBuilderConfirm
) -> hyper::http::Result<Request<Body>>
{
    let keyload_msg_link = Address::from_str(&keyload_msg_link_str).expect("Address::from_str() returned error");
    subscriber_mngr.register_keyload_msg(&keyload_msg_link).expect("[fn register_keyload_msg] register_keyload_msg err");

    println_subscription_details(
        &subscriber_mngr.subscriber.as_ref().unwrap(),
        &keyload_msg_link,
        "Keyload Message",
        "Keyload  msg",
    );

    confirm_req_builder.keyload_registration()
}

struct CmdProcessor<'a> {
    command_fetcher: CommandFetcher<'a>,
    iota_bridge_url: String,
    vfs_fat_path: Option<String>,
}

impl<'a> CmdProcessor<'a> {
    pub fn new(iota_bridge_url: &'a str, vfs_fat_path: Option<String>) -> CmdProcessor<'a> {
        CmdProcessor {
            iota_bridge_url: String::from(iota_bridge_url),
            command_fetcher: CommandFetcher::new(
                Some(CommandFetcherOptions{ http_url: iota_bridge_url })
            ),
            vfs_fat_path,
        }
    }
}

#[async_trait(?Send)]
impl<'a> SensorFunctions for CmdProcessor<'a> {
    type SubscriberManager = SubscriberManagerPlainTextWalletHttpClientEspRs;

    fn get_iota_bridge_url(&self) -> &str {
        self.iota_bridge_url.as_str()
    }

    async fn subscribe_to_channel(
        &self, announcement_link_str: &str, subscriber_mngr: &mut Self::SubscriberManager, confirm_req_builder: &RequestBuilderConfirm
    ) -> hyper::http::Result<Request<Body>>
    {
        subscribe_to_channel(announcement_link_str, subscriber_mngr, confirm_req_builder).await
    }

    async fn send_content_as_msg(
        &self, message_key: String, subscriber: &mut Self::SubscriberManager, confirm_req_builder: &RequestBuilderConfirm
    ) -> hyper::http::Result<Request<Body>>
    {
        send_content_as_msg(message_key, subscriber, confirm_req_builder).await
    }

    async fn register_keyload_msg(
        &self, keyload_msg_link_str: &str, subscriber_mngr: &mut Self::SubscriberManager, confirm_req_builder: &RequestBuilderConfirm
    ) -> hyper::http::Result<Request<Body>>
    {
        register_keyload_msg(keyload_msg_link_str, subscriber_mngr, confirm_req_builder).await
    }

    fn println_subscriber_status<'b>(
        &self, subscriber_manager: &Self::SubscriberManager, confirm_req_builder: &RequestBuilderConfirm
    ) -> hyper::http::Result<Request<Body>>
    {
        println_subscriber_status(subscriber_manager, confirm_req_builder)
    }

    async fn clear_client_state<'b>(
        &self, subscriber_manager: &mut Self::SubscriberManager, confirm_req_builder: &RequestBuilderConfirm
    ) -> hyper::http::Result<Request<Body>>
    {
        clear_client_state(subscriber_manager, confirm_req_builder).await
    }
}

#[async_trait(?Send)]
impl<'a> CommandProcessor for CmdProcessor<'a> {
    async fn fetch_next_command(&self) -> Result<(Command, Vec<u8>)> {
        self.command_fetcher.fetch_next_command()
    }

    async fn send_confirmation(&self, confirmation_request: Request<Body>) -> Result<()> {
        self.command_fetcher.send_confirmation(confirmation_request).await
    }

    async fn process_command(&self, command: Command, buffer: Vec<u8>) -> Result<Request<Body>> {
        let client = HttpClient::new(Some(HttpClientOptions{ http_url: self.iota_bridge_url.as_str() }));
        let (mut subscriber, mut vfs_fat_handle) =
            create_subscriber::<HttpClient, PlainTextWallet>(client, self.vfs_fat_path.clone()).await?;

        print_heap_info();

        let confirmation_request = process_sensor_commands(
            self, &mut subscriber, command, buffer
        ).await.expect("Error on processing sensor commands");

        log::debug!("[fn process_command]  Safe subscriber client_status to disk");
        subscriber.safe_client_status_to_disk().await?;
        log::debug!("[fn process_command]  vfs_fat_handle.drop_filesystem()");
        vfs_fat_handle.drop_filesystem()?;

        confirmation_request.ok_or(anyhow!("No confirmation_request received"))
    }
}

pub async fn process_main_esp_rs(lorawan_send_callback: send_request_via_lorawan_t, iota_bridge_url: &str, vfs_fat_path: Option<String>) -> Result<()> {
    log::debug!("[fn process_main_esp_rs] process_main() entry");

    print_heap_info();

    log::info!("[fn process_main_esp_rs] Using iota-bridge url: {}", iota_bridge_url);
    let command_processor = CmdProcessor::new(iota_bridge_url, vfs_fat_path);
    run_command_fetch_loop(
        command_processor,
        Some(
            CommandFetchLoopOptions{
                confirm_fetch_wait_sec: SUSEE_CONST_COMMAND_CONFIRM_FETCH_WAIT_SEC
            })
    ).await
}


pub async fn process_main_esp_rs_wifi(wifi_ssid: &str, wifi_pass: &str, iota_bridge_url: &str, vfs_fat_path: Option<String>) -> Result<()> {
    log::debug!("[fn process_main_esp_rs] process_main() entry");

    print_heap_info();

    log::debug!("[fn process_main_esp_rs] init_wifi");
    let _wifi_hdl = init_wifi(wifi_ssid, wifi_pass)?;

    log::info!("[fn process_main_esp_rs] Using iota-bridge url: {}", iota_bridge_url);
    let command_processor = CmdProcessor::new(iota_bridge_url, vfs_fat_path);
    run_command_fetch_loop(
        command_processor,
        Some(
            CommandFetchLoopOptions{
                confirm_fetch_wait_sec: SUSEE_CONST_COMMAND_CONFIRM_FETCH_WAIT_SEC
            })
    ).await
}