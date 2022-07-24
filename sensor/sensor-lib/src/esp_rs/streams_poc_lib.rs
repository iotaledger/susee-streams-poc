use super::{
    http_client_smol_esp_rs::{
        HttpClient,
        HttpClientOptions,
    },
};

#[cfg(feature = "esp_idf")]
use super::{
    vfs_fat_fs_tools::{
        setup_vfs_fat_filesystem,
        drop_vfs_fat_filesystem,
        BASE_PATH,
    }
};

use streams_tools::{
    DummyWallet,
    SubscriberManager,
};

use iota_streams::app_channels::api::{
    tangle::{
        Bytes,
    }
};

use anyhow::{
    Result,
};

#[cfg(feature = "wifi")]
use super::{
    wifi_utils::init_wifi,
};


type ClientType = HttpClient;

type SubscriberManagerDummyWalletHttpClient = SubscriberManager<ClientType, DummyWallet>;

const TANGLE_PROXY_URL: &str = env!("SENSOR_MAIN_POC_TANGLE_PROXY_URL");

pub async fn send_message(message_bytes: &[u8]) -> Result<()>{

    #[cfg(feature = "wifi")]
        log::debug!("[fn - send_message] init_wifi");
    #[cfg(feature = "wifi")]
        let (_wifi_hdl, _client_settings) = init_wifi()?;

    log::debug!("[fn - send_message()] Creating DummyWallet");
    let wallet = DummyWallet{};

    #[cfg(feature = "esp_idf")]
        let vfs_fat_handle = setup_vfs_fat_filesystem()?;

    log::debug!("[fn - send_message()] Creating HttpClient");
    let client = HttpClient::new(Some(HttpClientOptions{ http_url: TANGLE_PROXY_URL }));
    log::debug!("[fn - send_message()] Creating subscriber");
    let mut subscriber= SubscriberManagerDummyWalletHttpClient::new(
        client,
        wallet,
        Some(String::from(BASE_PATH) + "/user-state-sensor.bin"),
    ).await;

    log::debug!("[fn - send_message()] subscriber created");

    log::info!("[fn - send_message()] Sending {} bytes payload\n", message_bytes.len());
    log::debug!("[fn - send_message()] Message text: {}", std::str::from_utf8(message_bytes).expect("Could not deserialize message bytes to utf8 str"));
    let msg_link = subscriber.send_signed_packet(&Bytes(message_bytes.to_vec())).await?;
    log::debug!("[fn - send_message()] Message sent: {}, tangle index: {:#}\n", msg_link, msg_link.to_msg_index());

    #[cfg(feature = "esp_idf")]
        {
            log::debug!("[fn - send_message()] Safe subscriber client_status to disk");
            subscriber.safe_client_status_to_disk().await?;
            log::debug!("[fn - send_message()] drop_vfs_fat_filesystem");
            drop_vfs_fat_filesystem(vfs_fat_handle)?;
        }

    Ok(())
}
