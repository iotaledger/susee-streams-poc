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

use streams_tools::{PlainTextWallet, SubscriberManager, SimpleWallet};

pub use esp_idf_sys::wl_handle_t;

use std::ffi::{
    CString
};
use streams_tools::subscriber_manager::ClientTTrait;

pub static BASE_PATH: &str = "/spiflash";
pub static SENSOR_STREAMS_USER_STATE_FILE_NAME: &str = "user-state-sensor.bin";

pub type SubscriberManagerPlainTextWalletHttpClientEspRs = SubscriberManager<HttpClientEspRs, PlainTextWallet>;

pub struct VfsFatHandle {
    pub is_vfs_managed_by_others: bool,
    pub base_path: String,
    pub wl_handle: wl_handle_t,
}

impl VfsFatHandle {
    pub fn new(opt_vfs_fat_path: Option<String>) -> Self {
        let base_path: String;
        let is_vfs_managed_by_others: bool;
        if let Some(vfs_fat_path) = opt_vfs_fat_path {
            base_path = vfs_fat_path;
            is_vfs_managed_by_others = true;
        } else {
            base_path = String::from(BASE_PATH);
            is_vfs_managed_by_others = false;
        }
        Self {
            is_vfs_managed_by_others,
            base_path,
            wl_handle: WL_INVALID_HANDLE,
        }
    }

    pub fn setup_filesystem(&mut self) -> Result<wl_handle_t> {
        log::debug!("[VfsFatHandle.setup_filesystem] Start");
        let mut ret_val: wl_handle_t = WL_INVALID_HANDLE;
        if !self.is_vfs_managed_by_others {
            log::debug!("[VfsFatHandle.setup_filesystem] self.is_vfs_managed_by_others == false. Creating esp_vfs_fat_mount_config_t");
            let mount_config = esp_vfs_fat_mount_config_t {
                max_files: 2,
                format_if_mount_failed: true,
                allocation_unit_size: CONFIG_WL_SECTOR_SIZE as usize,
                // disk_status_check_enable: true,  // Only available with esp_idf >= 5.0
            };

            let storage_str : CString = CString::new("storage").expect("CString::new for storage failed");
            let c_base_path: CString = CString::new(self.base_path.as_str()).expect("CString::new for self.base_path failed");

            esp!(unsafe {esp_vfs_fat_spiflash_mount(c_base_path.as_ptr(), storage_str.as_ptr(), &mount_config, &mut ret_val)})?;
            self.wl_handle = ret_val;
        } else {
            log::debug!("[VfsFatHandle.setup_filesystem] self.is_vfs_managed_by_others == true. State of this struct remains unchanged.");
        }
        Ok(ret_val)
    }

    pub fn drop_filesystem(&mut self) -> Result<()> {
        log::debug!("[VfsFatHandle.drop_filesystem] Start");
        if !self.is_vfs_managed_by_others {
            log::debug!("[VfsFatHandle.drop_filesystem] self.is_vfs_managed_by_others == false");
            let c_base_path: CString = CString::new(self.base_path.as_str()).expect("CString::new for self.base_path failed");
            log::debug!("[VfsFatHandle.drop_filesystem] base_path created");
            esp!(unsafe {esp_vfs_fat_spiflash_unmount(c_base_path.as_ptr(), self.wl_handle)})?;
            log::debug!("[VfsFatHandle.drop_filesystem] esp_vfs_fat_spiflash_unmount finished - set self.wl_handle = WL_INVALID_HANDLE");
            self.wl_handle = WL_INVALID_HANDLE;
        } else {
            log::debug!("[VfsFatHandle.drop_filesystem] self.is_vfs_managed_by_others == true. State of this struct remains unchanged.");
        }
        log::debug!("[VfsFatHandle.drop_filesystem] returning OK");
        Ok(())
    }
}

pub async fn create_subscriber<ClientT, WalletT>(client: ClientT, opt_vfs_fat_path: Option<String>) -> Result<(SubscriberManager<ClientT, WalletT>, VfsFatHandle)>
where
    ClientT: ClientTTrait,
    WalletT: SimpleWallet
{
    log::debug!("[fn - create_subscriber()] setup_filesystem");
    let mut vfs_fat_handle = VfsFatHandle::new(opt_vfs_fat_path);
    vfs_fat_handle.setup_filesystem()?;

    log::debug!("[fn - create_subscriber()] Creating Wallet");
    let wallet_path = vfs_fat_handle.base_path.clone() + "/wallet_sensor.txt";
    let wallet = WalletT::new(wallet_path.as_str());

    log::debug!("[fn - create_subscriber()] Creating HttpClient");
    log::debug!("[fn - create_subscriber()] Creating subscriber");
    let subscriber= SubscriberManager::<ClientT, WalletT>::new(
        client,
        wallet,
        Some(vfs_fat_handle.base_path.clone() + "/" + SENSOR_STREAMS_USER_STATE_FILE_NAME),
    ).await;

    log::debug!("[fn - create_subscriber()] subscriber created");
    Ok((subscriber, vfs_fat_handle))
}
