use super::{
    CommandFetcherSocket,
    CommandFetcherSocketOptions,
    CommandFetcherBufferCb,
    CommandFetcherBufferCbOptions,
    StreamsTransportSocketEspRs,
    StreamsTransportSocketEspRsOptions,
    streams_transport_via_buffer_cb::StreamsTransportViaBufferCallback,
    esp32_subscriber_tools::{
        create_subscriber,
    },
    wifi_utils::init_wifi,
};

use crate::{
    command_fetcher::CommandFetcher,
    streams_poc_lib_api_types::send_request_via_lorawan_t,
    request_via_buffer_cb::RequestViaBufferCallbackOptions,
};

use payloads::{
    Message,
    get_message_bytes,
};

use susee_tools::SUSEE_CONST_COMMAND_CONFIRM_FETCH_WAIT_SEC;

use streams_tools::{
    subscriber_manager::get_public_key_str,
    binary_persist::{
        Subscription,
        SubscriberStatus,
        Command,
    },
    http::http_protocol_confirm::RequestBuilderConfirm,
    remote::command_processor::{
        CommandProcessor,
        SensorFunctions,
        CommandFetchLoopOptions,
        run_command_fetch_loop,
        process_sensor_commands,
    },
    PlainTextWallet,
    StreamsTransport,
    SubscriberManager
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
use esp_idf_svc::wifi::EspWifi;

fn print_heap_info() {
    unsafe {
        let free_mem = esp_idf_sys::heap_caps_get_free_size(
            esp_idf_sys::MALLOC_CAP_8BIT
        );

        log::info!("[fn print_heap_info] heap_caps_get_free_size(MALLOC_CAP_8BIT): {}", free_mem);
    }
}

struct CmdProcessor<CmdFetchT, StreamsTransportT>
where
    CmdFetchT: CommandFetcher,
    StreamsTransportT: StreamsTransport,
    StreamsTransportT::Options: Clone,
{
    command_fetcher: CmdFetchT,
    streams_transport_opt: StreamsTransportT::Options,
    vfs_fat_path: Option<String>,
}

impl<CmdFetchT, StreamsTransportT> CmdProcessor<CmdFetchT, StreamsTransportT>
    where
        CmdFetchT: CommandFetcher,
        StreamsTransportT: StreamsTransport,
        StreamsTransportT::Options: Clone,
{
    fn println_subscription_details(subscriber: &Subscriber<StreamsTransportT>, subscription_link: &Address, comment: &str, key_name: &str) {
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
        subscriber_manager: &<Self as SensorFunctions>::SubscriberManager,
        confirm_req_builder: &RequestBuilderConfirm
    ) -> hyper::http::Result<Request<Body>>
    {
        let mut ret_val: Option<Request<Body>> = None;
        if let Some(subscriber) = &subscriber_manager.subscriber {
            if let Some(subscription_link) = subscriber_manager.subscription_link {
                Self::println_subscription_details(&subscriber, &subscription_link, "A subscription with the following details has already been created", "Subscription");

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
        subscriber_manager: &mut <Self as SensorFunctions>::SubscriberManager,
        confirm_req_builder: &RequestBuilderConfirm
    ) -> hyper::http::Result<Request<Body>>
    {
        subscriber_manager.clear_client_state().await.expect("subscriber_manager.clear_client_state() returned error");
        confirm_req_builder.clear_client_state()
    }

    pub async fn send_content_as_msg(
        message_key: String,
        subscriber: &mut <Self as SensorFunctions>::SubscriberManager,
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
        subscriber_mngr: &mut <Self as SensorFunctions>::SubscriberManager,
        confirm_req_builder: &RequestBuilderConfirm
    ) -> hyper::http::Result<Request<Body>>
    {
        let ann_address = Address::from_str(&announcement_link_str).expect("Address::from_str() returned error");
        let sub_msg_link = subscriber_mngr.subscribe(&ann_address).await.expect("subscriber_mngr::subscribe() returned error");

        let subscriber = subscriber_mngr.subscriber.as_ref().unwrap();

        Self::println_subscription_details(
            &subscriber,
            &sub_msg_link,
            "New subscription",
            "Subscription",
        );

        confirm_req_builder.subscription(sub_msg_link.to_string(), get_public_key_str(subscriber))
    }

    async fn register_keyload_msg(
        keyload_msg_link_str: &str,
        subscriber_mngr: &mut <Self as SensorFunctions>::SubscriberManager,
        confirm_req_builder: &RequestBuilderConfirm
    ) -> hyper::http::Result<Request<Body>>
    {
        let keyload_msg_link = Address::from_str(&keyload_msg_link_str).expect("Address::from_str() returned error");
        subscriber_mngr.register_keyload_msg(&keyload_msg_link).expect("[fn register_keyload_msg] register_keyload_msg err");

        Self::println_subscription_details(
            &subscriber_mngr.subscriber.as_ref().unwrap(),
            &keyload_msg_link,
            "Keyload Message",
            "Keyload  msg",
        );

        confirm_req_builder.keyload_registration()
    }
}

impl<CmdFetchT, StreamsTransportT> CmdProcessor<CmdFetchT, StreamsTransportT>
    where
        CmdFetchT: CommandFetcher,
        StreamsTransportT: StreamsTransport,
        StreamsTransportT::Options: Clone,
{
    pub fn new<>(vfs_fat_path: Option<String>, command_fetch_opt: CmdFetchT::Options, streams_transport_opt: StreamsTransportT::Options) -> CmdProcessor<CmdFetchT, StreamsTransportT> {
        CmdProcessor {
            command_fetcher: CmdFetchT::new(
                Some(command_fetch_opt)
            ),
            streams_transport_opt,
            vfs_fat_path,
        }
    }
}

#[async_trait(?Send)]
impl<CmdFetchT, StreamsTransportT> SensorFunctions for CmdProcessor<CmdFetchT, StreamsTransportT>
    where
        CmdFetchT: CommandFetcher,
        StreamsTransportT: StreamsTransport,
        StreamsTransportT::Options: Clone,
{
    type SubscriberManager = SubscriberManager<StreamsTransportT, PlainTextWallet>;

    fn get_iota_bridge_url(&self) -> String {
        if let Some(url) = self.command_fetcher.get_iota_bridge_url() {
            url.clone()
        } else {
            "".to_string()
        }
    }

    async fn subscribe_to_channel(
        &self, announcement_link_str: &str, subscriber_mngr: &mut Self::SubscriberManager, confirm_req_builder: &RequestBuilderConfirm
    ) -> hyper::http::Result<Request<Body>>
    {
        Self::subscribe_to_channel(announcement_link_str, subscriber_mngr, confirm_req_builder).await
    }

    async fn send_content_as_msg(
        &self, message_key: String, subscriber: &mut Self::SubscriberManager, confirm_req_builder: &RequestBuilderConfirm
    ) -> hyper::http::Result<Request<Body>>
    {
        Self::send_content_as_msg(message_key, subscriber, confirm_req_builder).await
    }

    async fn register_keyload_msg(
        &self, keyload_msg_link_str: &str, subscriber_mngr: &mut Self::SubscriberManager, confirm_req_builder: &RequestBuilderConfirm
    ) -> hyper::http::Result<Request<Body>>
    {
        Self::register_keyload_msg(keyload_msg_link_str, subscriber_mngr, confirm_req_builder).await
    }

    fn println_subscriber_status<'b>(
        &self, subscriber_manager: &Self::SubscriberManager, confirm_req_builder: &RequestBuilderConfirm
    ) -> hyper::http::Result<Request<Body>>
    {
        Self::println_subscriber_status(subscriber_manager, confirm_req_builder)
    }

    async fn clear_client_state<'b>(
        &self, subscriber_manager: &mut Self::SubscriberManager, confirm_req_builder: &RequestBuilderConfirm
    ) -> hyper::http::Result<Request<Body>>
    {
        Self::clear_client_state(subscriber_manager, confirm_req_builder).await
    }
}

#[async_trait(?Send)]
impl<CmdFetchT, StreamsTransportT> CommandProcessor for CmdProcessor<CmdFetchT, StreamsTransportT>where
    CmdFetchT: CommandFetcher,
    StreamsTransportT: StreamsTransport,
    StreamsTransportT::Options: Clone,
{
    async fn fetch_next_command(&self) -> Result<(Command, Vec<u8>)> {
        self.command_fetcher.fetch_next_command().await
    }

    async fn send_confirmation(&self, confirmation_request: Request<Body>) -> Result<()> {
        self.command_fetcher.send_confirmation(confirmation_request).await
    }

    async fn process_command(&self, command: Command, buffer: Vec<u8>) -> Result<Request<Body>> {
        let transport = StreamsTransportT::new(
            Some(self.streams_transport_opt.clone())
        );

        let (mut subscriber, mut vfs_fat_handle) =
            create_subscriber::<StreamsTransportT, PlainTextWallet>(transport, self.vfs_fat_path.clone()).await?;

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

pub async fn process_main_esp_rs(
    lorawan_send_callback: send_request_via_lorawan_t,
    vfs_fat_path: Option<String>,
    p_caller_user_data: *mut cty::c_void,
) -> Result<()>
{
    log::debug!("[fn process_main_esp_rs] process_main() entry");

    print_heap_info();

    log::info!("[fn process_main_esp_rs] Using callback functions to send and receive binary packages");
    let request_via_callback_opt = RequestViaBufferCallbackOptions{
        send_callback: lorawan_send_callback,
        p_caller_user_data: p_caller_user_data,
    };
    let command_processor =
        CmdProcessor::<CommandFetcherBufferCb, StreamsTransportViaBufferCallback>::new(
            vfs_fat_path,
            CommandFetcherBufferCbOptions{ buffer_cb: request_via_callback_opt.clone()},
            request_via_callback_opt
    );
    run_command_fetch_loop(
        command_processor,
        Some(
            CommandFetchLoopOptions{
                confirm_fetch_wait_sec: SUSEE_CONST_COMMAND_CONFIRM_FETCH_WAIT_SEC
            })
    ).await
}

pub async fn process_main_esp_rs_lwip(
    iota_bridge_url: &str,
    vfs_fat_path: Option<String>,
    opt_wifi_ssid: Option<String>,
    opt_wifi_pass: Option<String>,
) -> Result<()> {
    log::debug!("[fn process_main_esp_rs] process_main() entry");

    print_heap_info();

    let _wifi_hdl: Box<EspWifi<'static>>;

    log::debug!("[fn process_main_esp_rs] init_wifi");
    if let Some(wifi_ssid) = opt_wifi_ssid {
        let wifi_pass = opt_wifi_pass.expect("[fn process_main_esp_rs_lwip()] wifi_ssid is specified but no wifi_pass has been provided.\
         You always need to provide both wifi_ssid and wifi_pass or set wifi_ssid to NULL");
        _wifi_hdl = init_wifi(wifi_ssid.as_str(), wifi_pass.as_str())?;
    }

    log::info!("[fn process_main_esp_rs] Using iota-bridge url: {}", iota_bridge_url);

    let command_processor =
        CmdProcessor::<CommandFetcherSocket, StreamsTransportSocketEspRs>::new(
            vfs_fat_path,
            CommandFetcherSocketOptions{
                http_url: iota_bridge_url
            },
            StreamsTransportSocketEspRsOptions{
                http_url: iota_bridge_url.to_string()
            }
    );
    run_command_fetch_loop(
        command_processor,
        Some(
            CommandFetchLoopOptions{
                confirm_fetch_wait_sec: SUSEE_CONST_COMMAND_CONFIRM_FETCH_WAIT_SEC
            })
    ).await
}