use anyhow::{
    Result
};

use esp_idf_sys::{
    esp_vfs_fat_spiflash_mount,
    esp_vfs_fat_spiflash_unmount,
    esp_vfs_fat_mount_config_t,
    CONFIG_WL_SECTOR_SIZE,
    WL_INVALID_HANDLE,
    esp,
};

pub use esp_idf_sys::wl_handle_t;

use std::ffi::{
    CString
};

pub static BASE_PATH: &str = "/spiflash";

pub fn setup_vfs_fat_filesystem() -> Result<wl_handle_t> {
    log::debug!("[Sensor] Starting setup_vfs_fat_filesystem");

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
    log::debug!("[Sensor] Starting drop_vfs_fat_filesystem");
    let base_path: CString = CString::new(BASE_PATH).expect("CString::new for BASE_PATH failed");
    esp!(unsafe {esp_vfs_fat_spiflash_unmount(base_path.as_ptr(), s_wl_handle)})?;
    Ok(())
}