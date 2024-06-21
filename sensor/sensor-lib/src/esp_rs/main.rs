use core::str::FromStr;

use std::{
    rc::Rc,
    cell::RefCell,
};

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

use async_trait::async_trait;

use esp_idf_svc::wifi::EspWifi;

use lets::{
    message::TransportMessage,
    transport::Transport,
};

use streams::{
    Address,
    User,
};

use streams_tools::{
    subscriber_manager::{
        get_public_key_str,
    },
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
    SubscriberManager,
};

use susee_tools::SUSEE_CONST_COMMAND_CONFIRM_FETCH_WAIT_SEC;

use payloads::{
    Message,
    get_message_bytes,
};

use crate::{
    command_fetcher::CommandFetcher,
    streams_poc_lib_api_types::{
        send_request_via_lorawan_t,
    },
    request_via_buffer_cb::RequestViaBufferCallbackOptions,
};

use super::{
    CommandFetcherSocket,
    CommandFetcherSocketOptions,
    CommandFetcherBufferCb,
    CommandFetcherBufferCbOptions,
    StreamsTransportSocketEspRs,
    StreamsTransportSocketEspRsOptions,
    streams_transport_via_buffer_cb::StreamsTransportViaBufferCallback,
    wifi_utils::init_wifi,
    client_data_persistence::{
        create_subscriber,
        ClientDataPersistence,
        ClientDataPersistenceOptions,
    },
};

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
    client_data_persistence: Rc<RefCell<ClientDataPersistence>>,
    dev_eui: String,
}

impl<CmdFetchT, StreamsTransportT> CmdProcessor<CmdFetchT, StreamsTransportT>
    where
        CmdFetchT: CommandFetcher,
        StreamsTransportT: StreamsTransport,
        StreamsTransportT::Options: Clone,
{
    fn println_subscription_details(
        user: &User<StreamsTransportT>,
        subscription_link: &Address,
        comment: &str,
        key_name: &str,
        initialization_cnt: u8
    ) {
        let public_key = get_public_key_str(user);
        log::info!(
            "[Sensor] {}:
         {} Link:     {}
              Tangle Index:     {:#}
         User public key: {}
         Initialization Count:  {}\n",
            comment,
            key_name,
            subscription_link.to_string(),
            hex::encode(subscription_link.to_msg_index()),
            public_key,
            initialization_cnt,
        );
    }
}

#[async_trait(?Send)]
impl<TSR, CmdFetchT, StreamsTransportT> SensorFunctions for CmdProcessor<CmdFetchT, StreamsTransportT>
    where
        CmdFetchT: CommandFetcher,
        StreamsTransportT: StreamsTransport + for <'a> Transport<'a, Msg = TransportMessage, SendResponse = TSR>,
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

    fn get_dev_eui(&self) -> String {
        self.dev_eui.clone()
    }

    fn println_subscriber_status<'a> (
        &self,
        subscriber_manager: &<Self as SensorFunctions>::SubscriberManager,
        confirm_req_builder: &RequestBuilderConfirm
    ) -> hyper::http::Result<Request<Body>>
    {
        let mut ret_val: Option<Request<Body>> = None;
        if let Some(user) = &subscriber_manager.user {
            if let Some(subscription_link) = subscriber_manager.subscription_link {
                Self::println_subscription_details(
                    &user,
                    &subscription_link,
                    "A subscription with the following details exists",
                    "Subscription",
                    subscriber_manager.get_initialization_cnt(),
                );

                let mut previous_message_link= String::from("");
                if let Some(prev_msg_link) = subscriber_manager.prev_msg_link {
                    previous_message_link = prev_msg_link.to_string();
                }

                ret_val = Some(
                    confirm_req_builder.subscriber_status(
                        previous_message_link,
                        Subscription {
                            subscription_link: subscription_link.to_string(),
                            pup_key: get_public_key_str(user),
                            initialization_cnt: subscriber_manager.get_initialization_cnt()
                        })?
                );
            }
        }
        if ret_val.is_none() {
            log::info!("No subscription found.");
            let to_send = SubscriberStatus::default();
            ret_val = Some( confirm_req_builder.subscriber_status(to_send.previous_message_link, to_send.subscription)?);
        }

        if let Some(prev_msg_link) = subscriber_manager.prev_msg_link {
            log::info!(
                "[Sensor] Previous message:
         Prev msg link:     {}
             Tangle Index:     {:#}",
                prev_msg_link.to_string(),
                hex::encode(prev_msg_link.to_msg_index())
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
        &self,
        subscriber_manager: &mut <Self as SensorFunctions>::SubscriberManager,
        confirm_req_builder: &RequestBuilderConfirm
    ) -> hyper::http::Result<Request<Body>>
    {
        subscriber_manager.clear_client_state().await.expect("subscriber_manager.clear_client_state() returned error");
        confirm_req_builder.clear_client_state()
    }

    async fn send_content_as_msg_in_endless_loop(
        &self,
        message_key: String,
        user: &mut <Self as SensorFunctions>::SubscriberManager,
        confirm_req_builder: &RequestBuilderConfirm
    ) -> hyper::http::Result<Request<Body>>
    {
        // TODO: We do only run one cycle here as this function is seldomly used for ESP32
        //       remote control.
        let message_bytes = get_message_bytes(Message::from(message_key.as_str()));
        log::info!("[fn send_content_as_msg] Sending {} bytes payload\n", message_bytes.len());
        log::debug!("[fn send_content_as_msg] - send_content_as_msg()] Message text: {}", std::str::from_utf8(message_bytes).expect("Could not deserialize message bytes to utf8 str"));
        user.send_signed_packet(&message_bytes.to_vec()).await.expect("user.send_signed_packet() returned error");
        confirm_req_builder.send_messages_in_endless_loop()
    }

    async fn subscribe_to_channel(
        &self,
        announcement_link_str: &str,
        subscriber_mngr: &mut <Self as SensorFunctions>::SubscriberManager,
        confirm_req_builder: &RequestBuilderConfirm
    ) -> hyper::http::Result<Request<Body>>
    {
        let ann_address = Address::from_str(&announcement_link_str).expect("Address::from_str() returned error");
        let sub_msg_link = subscriber_mngr.subscribe(ann_address.clone()).await.expect("subscriber_mngr::subscribe() returned error");

        let user = subscriber_mngr.user.as_ref().unwrap();

        Self::println_subscription_details(
            &user,
            &sub_msg_link,
            "New subscription",
            "Subscription",
            subscriber_mngr.get_initialization_cnt(),
        );

        confirm_req_builder.subscription(
            sub_msg_link.to_string(),
            get_public_key_str(user),
            subscriber_mngr.get_initialization_cnt()
        )
    }

    async fn register_keyload_msg(
        &self,
        keyload_msg_link_str: &str,
        subscriber_mngr: &mut <Self as SensorFunctions>::SubscriberManager,
        confirm_req_builder: &RequestBuilderConfirm
    ) -> hyper::http::Result<Request<Body>>
    {
        let keyload_msg_link = Address::from_str(&keyload_msg_link_str).expect("Address::from_str() returned error");
        subscriber_mngr.register_keyload_msg(&keyload_msg_link).await
            .expect("[fn register_keyload_msg] register_keyload_msg err");

        Self::println_subscription_details(
            &subscriber_mngr.user.as_ref().unwrap(),
            &keyload_msg_link,
            "Keyload Message",
            "Keyload  msg",
            subscriber_mngr.get_initialization_cnt(),
        );

        confirm_req_builder.keyload_registration()
    }

    async fn send_random_msg_in_endless_loop(&self, _msg_size: usize, _subscriber: &mut Self::SubscriberManager, _confirm_req_builder: &RequestBuilderConfirm) -> hyper::http::Result<Request<Body>> {
        // TODO: This is not implemented as this function is not used for ESP32
        //       remote control.
        todo!()
    }

    async fn dev_eui_handshake(&self, confirm_req_builder: &RequestBuilderConfirm) -> hyper::http::Result<Request<Body>> {
        confirm_req_builder.dev_eui_handshake(
            self.dev_eui.clone()
        )
    }
}

impl<CmdFetchT, StreamsTransportT> CmdProcessor<CmdFetchT, StreamsTransportT>
    where
        CmdFetchT: CommandFetcher,
        StreamsTransportT: StreamsTransport,
        StreamsTransportT::Options: Clone,
{
    pub fn new<>(
        dev_eui: &str,
        client_data_persist_opt: ClientDataPersistenceOptions,
        command_fetch_opt: CmdFetchT::Options,
        streams_transport_opt: StreamsTransportT::Options
    ) -> CmdProcessor<CmdFetchT, StreamsTransportT>
    {
        let client_data_persistence= Rc::new(RefCell::new(
            ClientDataPersistence::new(client_data_persist_opt.clone())
        ));
        CmdProcessor {
            command_fetcher: CmdFetchT::new(
                Some(command_fetch_opt)
            ),
            streams_transport_opt,
            client_data_persistence,
            dev_eui: dev_eui.to_string(),
        }
    }
}

#[async_trait(?Send)]
impl<TSR, CmdFetchT, StreamsTransportT> CommandProcessor for CmdProcessor<CmdFetchT, StreamsTransportT> where
    CmdFetchT: CommandFetcher,
    StreamsTransportT: StreamsTransport + for <'a> Transport<'a, Msg = TransportMessage, SendResponse = TSR>,
    StreamsTransportT::Options: Clone,
{
    fn get_dev_eui(&self) -> String {
        self.dev_eui.clone()
    }

    async fn fetch_next_command(&self) -> Result<(Command, Vec<u8>)> {
        self.command_fetcher.fetch_next_command().await
    }

    async fn send_confirmation(&self, confirmation_request: Request<Body>) -> Result<()> {
        self.command_fetcher.send_confirmation(confirmation_request).await
    }

    async fn process_command(&mut self, command: Command, buffer: Vec<u8>) -> Result<Request<Body>> {
        log::debug!("[fn process_command]  Preparing client_data_persistence");
        self.client_data_persistence.borrow_mut().prepare()?;

        log::debug!("[fn process_command]  Creating user");
        let mut user=
            create_subscriber::<StreamsTransportT, PlainTextWallet>(
                Some(self.streams_transport_opt.clone()),
                self.client_data_persistence.clone()
            ).await?;

        print_heap_info();

        let confirmation_request = process_sensor_commands(
            self, &mut user, command, buffer
        ).await.expect("Error on processing sensor commands");

        log::debug!("[fn process_command]  user.save_client_state()");
        user.save_client_state().await?;
        log::debug!("[fn process_command]  client_data_persistence.flush_resources()");
        self.client_data_persistence.borrow_mut().flush_resources()?;

        confirmation_request.ok_or(anyhow!("No confirmation_request received"))
    }
}

pub async fn process_main_esp_rs(
    lorawan_send_callback: send_request_via_lorawan_t,
    dev_eui: &str,
    client_data_persist_opt: ClientDataPersistenceOptions,
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
            dev_eui,
            client_data_persist_opt,
            CommandFetcherBufferCbOptions{
                buffer_cb: request_via_callback_opt.clone(),
                dev_eui_handshake_first: true,
                dev_eui: dev_eui.to_string(),
            },
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
    dev_eui: &str,
    client_data_persist_opt: ClientDataPersistenceOptions,
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
            dev_eui,
            client_data_persist_opt,
            CommandFetcherSocketOptions{
                http_url: iota_bridge_url.to_string(),
                dev_eui_handshake_first: true,
                dev_eui: dev_eui.to_string(),
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