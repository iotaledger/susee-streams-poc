use anyhow::{
    Result,
};

use streams_tools::{
    PlainTextWallet,
};

use super::{
    super::{
        streams_transport_via_buffer_cb::{
            StreamsTransportViaBufferCallback,
        },
        client_data_persistence::{
            create_subscriber,
            ClientDataPersistence,
        },
        super::{
            request_via_buffer_cb::RequestViaBufferCallbackOptions,
            streams_poc_lib_api_types::{
                send_request_via_lorawan_t,
            },
        },
    },
};
use crate::esp_rs::client_data_persistence::ClientDataPersistenceOptions;

pub async fn send_message(
    message_bytes: &[u8],
    lorawan_send_callback: send_request_via_lorawan_t,
    client_data_persist_opt: ClientDataPersistenceOptions,
    p_caller_user_data: *mut cty::c_void
) -> Result<()>{
    log::debug!("[fn send_message()] Creating subscriber");
    let client_data_persistence = ClientDataPersistence::prepared_new(client_data_persist_opt)?;

    let mut subscriber =
        create_subscriber::<StreamsTransportViaBufferCallback, PlainTextWallet>(
            Some(RequestViaBufferCallbackOptions { send_callback: lorawan_send_callback, p_caller_user_data}),
            client_data_persistence.clone()
    ).await?;

    log::info!("[fn send_message()] Sending {} bytes payload\n", message_bytes.len());
    log::debug!("[fn send_message()] Message text: {}", std::str::from_utf8(message_bytes).expect("Could not deserialize message bytes to utf8 str"));
    match subscriber.send_signed_packet(&message_bytes.to_vec()).await {
        Ok(msg_link) => {
            log::debug!("[fn send_message()] Message sent: {}, tangle index: {:#}\n", msg_link, hex::encode(msg_link.to_msg_index()));
        },
        Err(e) => {
            log::error!("[fn send_message()] Error while sending Message: {}", e);
        }
    }
    log::debug!("[fn send_message()] Safe subscriber client_status to disk");
    subscriber.save_client_state().await?;
    log::debug!("[fn send_message()] client_data_persistence.flush_resources()");
    client_data_persistence.borrow_mut().flush_resources()?;
    log::debug!("[fn send_message()] Return OK");
    Ok(())
}

pub async fn is_streams_channel_initialized(client_data_persist_opt: ClientDataPersistenceOptions) -> Result<bool> {
    log::debug!("[fn is_streams_channel_initialized()] \
            Creating client_data_persistence with following options:\n{}", client_data_persist_opt
    );
    let client_data_persistence = ClientDataPersistence::prepared_new(client_data_persist_opt)?;

    log::debug!("[fn is_streams_channel_initialized()] Creating subscriber");
    let subscriber = create_subscriber::<StreamsTransportViaBufferCallback, PlainTextWallet>(
        None,
        client_data_persistence.clone(),
    ).await?;

    log::debug!("[fn is_streams_channel_initialized()] Calling subscriber.is_channel_initialized()");
    let ret_val = subscriber.is_channel_initialized().await;

    log::debug!("[fn is_channel_initialized()] Calling client_data_persistence.flush_resources()");
    client_data_persistence.borrow_mut().flush_resources()?;

    ret_val
}