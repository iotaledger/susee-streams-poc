use anyhow::{
    Result
};
use super::{
    http_client_smol_esp_rs::{
        HttpClient as HttpClientEspRs,
    }
};

use esp_idf_sys::{
    esp_vfs_fat_spiflash_mount,
    esp_vfs_fat_spiflash_unmount,
    esp_vfs_fat_mount_config_t,
    CONFIG_WL_SECTOR_SIZE,
    WL_INVALID_HANDLE,
    esp,
};

use streams_tools::{DummyWallet, SubscriberManager, SimpleWallet};

pub use esp_idf_sys::wl_handle_t;

use std::ffi::{
    CString
};
use streams_tools::subscriber_manager::ClientTTrait;

pub static BASE_PATH: &str = "/spiflash";
pub static SENSOR_STREAMS_USER_STATE_FILE_NAME: &str = "user-state-sensor.bin";

pub type SubscriberManagerDummyWalletHttpClientEspRs = SubscriberManager<HttpClientEspRs, DummyWallet>;

pub(crate) const IOTA_BRIDGE_URL: &str = env!("SENSOR_MAIN_POC_IOTA_BRIDGE_URL");

pub fn setup_vfs_fat_filesystem() -> Result<wl_handle_t> {
    log::debug!("[fn setup_vfs_fat_filesystem] Starting setup_vfs_fat_filesystem");

    let mount_config = esp_vfs_fat_mount_config_t{
        max_files: 2,
        format_if_mount_failed: true,
        allocation_unit_size: CONFIG_WL_SECTOR_SIZE,
        disk_status_check_enable: true,
    };

    let mut ret_val: wl_handle_t = WL_INVALID_HANDLE;
    let storage_str : CString = CString::new("storage").expect("CString::new for storage failed");
    let base_path: CString = CString::new(BASE_PATH).expect("CString::new for BASE_PATH failed");

    esp!(unsafe {esp_vfs_fat_spiflash_mount(base_path.as_ptr(), storage_str.as_ptr(), &mount_config, &mut ret_val)})?;
    Ok(ret_val)
}

pub fn drop_vfs_fat_filesystem(s_wl_handle: wl_handle_t) -> Result<()> {
    log::debug!("[fn drop_vfs_fat_filesystem] Starting drop_vfs_fat_filesystem");
    let base_path: CString = CString::new(BASE_PATH).expect("CString::new for BASE_PATH failed");
    log::debug!("[fn drop_vfs_fat_filesystem] base_path created");
    esp!(unsafe {esp_vfs_fat_spiflash_unmount(base_path.as_ptr(), s_wl_handle)})?;
    log::debug!("[fn drop_vfs_fat_filesystem] esp_vfs_fat_spiflash_unmount finished - returning OK");
    Ok(())
}

pub async fn create_subscriber<ClientT, WalletT>(client: ClientT) -> Result<(SubscriberManager<ClientT, WalletT>, wl_handle_t)>
where
    ClientT: ClientTTrait,
    WalletT: SimpleWallet + Default
{
    log::debug!("[fn - create_subscriber()] Creating DummyWallet");
    let wallet = WalletT::default();

    let vfs_fat_handle = setup_vfs_fat_filesystem()?;

    log::debug!("[fn - create_subscriber()] Creating HttpClient");
    log::debug!("[fn - create_subscriber()] Creating subscriber");
    let subscriber= SubscriberManager::<ClientT, WalletT>::new(
        client,
        wallet,
        Some(String::from(BASE_PATH) + "/" + SENSOR_STREAMS_USER_STATE_FILE_NAME),
    ).await;

    log::debug!("[fn - create_subscriber()] subscriber created");
    Ok((subscriber, vfs_fat_handle))
}
