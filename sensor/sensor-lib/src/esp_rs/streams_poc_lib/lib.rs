use super::{
    super::{
        streams_transport_via_buffer_cb::{
            StreamsTransportViaBufferCallback,
        },
        esp32_subscriber_tools::{
            create_subscriber,
        }
    },
};

use iota_streams::app_channels::api::{
    tangle::{
        Bytes,
    }
};

use anyhow::{
    Result,
};

use crate::{
    streams_poc_lib_api_types::send_request_via_lorawan_t,
    request_via_buffer_cb::RequestViaBufferCallbackOptions,
};

use streams_tools::{
    PlainTextWallet,
    StreamsTransport
};

pub async fn send_message(
    message_bytes: &[u8],
    lorawan_send_callback: send_request_via_lorawan_t,
    vfs_fat_path: Option<String>,
    p_caller_user_data: *mut cty::c_void
) -> Result<()>{

    let streams_transport = StreamsTransportViaBufferCallback::new(
        Some(RequestViaBufferCallbackOptions { send_callback: lorawan_send_callback, p_caller_user_data})
    );
    let (mut subscriber, mut vfs_fat_handle) =
        create_subscriber::<StreamsTransportViaBufferCallback, PlainTextWallet>(streams_transport, vfs_fat_path).await?;

    log::info!("[fn - send_message()] Sending {} bytes payload\n", message_bytes.len());
    log::debug!("[fn - send_message()] Message text: {}", std::str::from_utf8(message_bytes).expect("Could not deserialize message bytes to utf8 str"));
    match subscriber.send_signed_packet(&Bytes(message_bytes.to_vec())).await {
        Ok(msg_link) => {
            log::debug!("[fn - send_message()] Message sent: {}, tangle index: {:#}\n", msg_link, msg_link.to_msg_index());
        },
        Err(e) => {
            log::error!("[fn - send_message()] Error while sending Message: {}", e);
        }
    }
    log::debug!("[fn - send_message()] Safe subscriber client_status to disk");
    subscriber.safe_client_status_to_disk().await?;
    log::debug!("[fn - send_message()] vfs_fat_handle.drop_filesystem()");
    vfs_fat_handle.drop_filesystem()?;
    log::debug!("[fn - send_message()] Return OK");
    Ok(())
}

pub async fn is_streams_channel_initialized(vfs_fat_path: Option<String>) -> Result<bool>{
    log::debug!("[fn - is_streams_channel_initialized()] Creating subscriber");
    let client = StreamsTransportViaBufferCallback::new(None);
    let (subscriber, mut vfs_fat_handle) =
        create_subscriber::<StreamsTransportViaBufferCallback, PlainTextWallet>(client, vfs_fat_path).await?;

    let ret_val = subscriber.subscription_link.is_some();
    log::debug!("[fn - is_streams_channel_initialized()] subscriber.subscription_link.is_some() == {}", ret_val);

    log::debug!("[fn - is_streams_channel_initialized()] vfs_fat_handle.drop_filesystem()");
    vfs_fat_handle.drop_filesystem()?;
    log::debug!("[fn - is_streams_channel_initialized()] returning Ok({})", ret_val);
    Ok(ret_val)
}