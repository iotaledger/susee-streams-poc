use std::{
    path::Path,
    rc::Rc,
    fmt,
    ptr,
    cell::RefCell,
        fs::{
        write,
        read,
        remove_file,
    }
};

use anyhow::{
    Result,
    bail
};

use streams_tools::{
    subscriber_manager::SubscriberPersistence,
    SubscriberManager,
    StreamsTransport,
    SimpleWallet
};

use crate::{
    streams_poc_lib_api_types::{
        VfsFatManagement,
        StreamsClientDataStorageType,
        streams_client_data_update_call_back_t,
        StreamsClientInitializationState
    }
};

use super::{
    esp32_vfs_fat_handle::{
        VfsFatHandle,
        mount_file_system
    }
};

pub static STREAMS_CLIENT_DATA_FILE_NAME: &str = "user-state-sensor.bin";

#[derive(Clone)]
pub struct ClientDataPersistenceOptions {
    pub vfs_fat_management: VfsFatManagement,
    pub streams_client_data_storage_type: StreamsClientDataStorageType,
    pub vfs_fat_path: Option<String>,
    pub client_initialization_state: StreamsClientInitializationState,
    pub latest_client_data_bytes: Option<Vec<u8>>,
    pub update_client_data_call_back: Option<streams_client_data_update_call_back_t>,
    pub p_update_call_back_caller_user_data: *mut cty::c_void
}

impl fmt::Display for ClientDataPersistenceOptions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut latest_client_data_bytes = "None".to_string();
        if let Some(data) = &self.latest_client_data_bytes {
            latest_client_data_bytes = format!("Length: {}", data.len());
        };
        write!(f, "ClientDataPersistenceOptions:
                vfs_fat_management: {:?}
                streams_client_data_storage_type: {:?}
                vfs_fat_path: {}
                client_initialization_state: {:?}
                latest_client_data_bytes: {}
                update_client_data_call_back: {}
                p_update_call_back_caller_user_data: {}
           ",
                self.vfs_fat_management,
                self.streams_client_data_storage_type,
                self.vfs_fat_path.clone().unwrap_or("None".to_string()),
                self.client_initialization_state,
                latest_client_data_bytes,
                if self.update_client_data_call_back.is_some() {"Some"} else {"None"},
                if self.p_update_call_back_caller_user_data == ptr::null_mut() {"Some"} else {"None"},
        )
    }
}

impl ClientDataPersistenceOptions {
    pub fn is_storage_call_back_client_initialized(&self) -> Result<bool> {
        match self.streams_client_data_storage_type {
            StreamsClientDataStorageType::CLIENT_DATA_STORAGE_CALL_BACK => {
                match self.client_initialization_state {
                    StreamsClientInitializationState::CLIENT_INIT_STATE_NOT_INITIALIZED => Ok(false),
                    StreamsClientInitializationState::CLIENT_INIT_STATE_INITIALIZED => Ok(true),
                    _ => bail!("ClientDataPersistenceOptions::client_initialization_state '{:?}' is not allowed \
                                if ClientDataPersistenceOptions::streams_client_data_storage_type == \
                                StreamsClientDataStorageType::CLIENT_DATA_STORAGE_CALL_BACK",
                                self.client_initialization_state)
                }
            }
            StreamsClientDataStorageType::CLIENT_DATA_STORAGE_VFS_FAT =>
                bail!("Function is_storage_call_back_client_initialized() must not be called \
                         if ClientDataPersistenceOptions::streams_client_data_storage_type == \
                         CLIENT_DATA_STORAGE_VFS_FAT"),
        }
    }

    pub fn validate(&self) -> Result<()> {
        self.validate_vfs_fat_path()?;
        if self.streams_client_data_storage_type == StreamsClientDataStorageType::CLIENT_DATA_STORAGE_CALL_BACK {
            self.validate_latest_client_data()?;
        }
        Ok(())
    }

    fn validate_vfs_fat_path(&self) -> Result<()> {
        match self.vfs_fat_management {
            VfsFatManagement::VFS_FAT_STREAMS_POC_LIB_MANAGED => {
                if self.vfs_fat_path.is_some() {
                    bail!("[fn validate_vfs_fat_handle_and_mount_fs] vfs_fat_path must be None if \
                    vfs_fat_management is VFS_FAT_STREAMS_POC_LIB_MANAGED")
                }
            }
            VfsFatManagement::VFS_FAT_APPLICATION_MANAGED => {
                if self.vfs_fat_path.is_none() {
                    bail!("[fn validate_vfs_fat_handle_and_mount_fs] vfs_fat_path must not be None if \
                    vfs_fat_management is VFS_FAT_APPLICATION_MANAGED")
                }
            }
        }
        Ok(())
    }

    fn validate_latest_client_data(&self) -> Result<()> {
        match self.client_initialization_state {
            StreamsClientInitializationState::CLIENT_INIT_STATE_UNKNOWN => {
                bail!("[fn validate_latest_client_data] client_initialization_state \
                    must be CLIENT_INIT_STATE_UNKNOWN if streams_client_data_storage_type \
                    is CLIENT_DATA_STORAGE_CALL_BACK")
            }
            StreamsClientInitializationState::CLIENT_INIT_STATE_NOT_INITIALIZED => {
                if self.latest_client_data_bytes.is_some() {
                    bail!("[fn validate_latest_client_data] latest_client_data_bytes \
                    must be None if client_initialization_state is \
                    CLIENT_INIT_STATE_NOT_INITIALIZED")
                }
            }
            StreamsClientInitializationState::CLIENT_INIT_STATE_INITIALIZED => {
                if let Some(client_data) = &self.latest_client_data_bytes {
                    if client_data.len() == 0 {
                        bail!("[fn validate_latest_client_data] latest_client_data_bytes \
                         must not have length of zero bytes if client_initialization_state \
                         is CLIENT_INIT_STATE_INITIALIZED");
                    }
                } else {
                    bail!("[fn validate_latest_client_data] latest_client_data_bytes must \
                    not be None if client_initialization_state is \
                    CLIENT_INIT_STATE_INITIALIZED");
                }
            }
        }
        Ok(())
    }
}

pub struct ClientDataPersistence {
    options: ClientDataPersistenceOptions,
    vfs_fat_handle: Option<VfsFatHandle>,
    latest_client_data_bytes: Option<Vec<u8>>,
    is_prepared: bool,
}

impl ClientDataPersistence {

    pub fn new(options: ClientDataPersistenceOptions) -> ClientDataPersistence {
        options.validate().expect("Error validating options");
        ClientDataPersistence{
            vfs_fat_handle: None,
            latest_client_data_bytes: options.latest_client_data_bytes.clone(),
            is_prepared: false,
            options,
        }
    }

    /// Returns a new instance after fn prepare() has been applied to it.
    /// See fn prepare() there for more details
    pub fn prepared_new(options: ClientDataPersistenceOptions) -> Result<Rc<RefCell<ClientDataPersistence>>> {
        log::debug!("[fn prepared_new()] Creating new prepared ClientDataPersistence");
        let mut cdp = Self::new(options);
        cdp.prepare()?;
        Ok(Rc::new(RefCell::new(cdp)))
    }

    /// Prepares this instance to read/write data.
    /// Use this function before any of the SubscriberPersistence functions are used.
    /// Use flush_resources() to flush all buffers and free all used handles.
    #[allow(unused_assignments)]
    pub fn prepare(&mut self) -> Result<()> {
        log::debug!("[fn prepare()] Preparing ClientDataPersistence");
        let vfs_fat_handle = mount_file_system(self.options.vfs_fat_path.clone())?;
        match self.options.streams_client_data_storage_type {
            StreamsClientDataStorageType::CLIENT_DATA_STORAGE_VFS_FAT => {
                self.latest_client_data_bytes = Some(Self::read_latest_client_state_from_file(&vfs_fat_handle)?);
            },
            StreamsClientDataStorageType::CLIENT_DATA_STORAGE_CALL_BACK => {
                match &self.latest_client_data_bytes {
                    None => {
                        self.latest_client_data_bytes = Self::clone_latest_client_data_from_options(&self.options)?;
                    }
                    Some(data) => {
                        log::debug!("[fn prepare()] latest_client_data_bytes exist. \
                                    No clone_latest_client_data_from_options() needed. \
                                    latest_client_data_bytes.len(): {}", data.len());
                    }
                }
            }
        }
        self.vfs_fat_handle = Some(vfs_fat_handle);
        self.is_prepared = true;
        log::debug!("[fn prepare()] Exiting fn");
        Ok(())
    }

    fn clone_latest_client_data_from_options(options: &ClientDataPersistenceOptions) -> Result<Option<Vec<u8>>> {
        let mut latest_client_data_bytes = None;
        if options.client_initialization_state == StreamsClientInitializationState::CLIENT_INIT_STATE_INITIALIZED {
            if let Some(client_data) = &options.latest_client_data_bytes {
                log::debug!("[fn clone_latest_client_data_from_options()] options.client_initialization_state \
                            is CLIENT_INIT_STATE_INITIALIZED.\n\
                            Cloning latest_client_data_bytes from options. Length is {}", client_data.len());
                latest_client_data_bytes = Some(client_data.clone());
            } else {
                log::warn!("[fn clone_latest_client_data_from_options()] options.latest_client_data_bytes \
                            is None although client_initialization_state is CLIENT_INIT_STATE_INITIALIZED");
            }
        } else {
            log::debug!("[fn clone_latest_client_data_from_options()] options.client_initialization_state is {:?}. \
                            Not cloning latest_client_data_bytes from options.", options.client_initialization_state );
        }

        Ok(latest_client_data_bytes)
    }

    fn compose_client_state_file_name(vfs_fat_handle: &VfsFatHandle) -> String {
        vfs_fat_handle.base_path.clone() + "/" + STREAMS_CLIENT_DATA_FILE_NAME
    }


    fn read_latest_client_state_from_file(vfs_fat_handle: &VfsFatHandle) -> Result<Vec<u8>> {
        let client_state_file_name = Self::compose_client_state_file_name(vfs_fat_handle);
        log::debug!("[fn read_latest_client_state_from_file()] client_state_file_name: '{}'", client_state_file_name);
        let new_path = Path::new(client_state_file_name.as_str());
        log::debug!("[fn read_latest_client_state_from_file()] new_path: '{}'", new_path.display());
        let path_exists = new_path.exists();
        log::debug!("[fn read_latest_client_state_from_file()] path_exists: '{}'", path_exists);
        if path_exists {
            log::debug!("[fn read_latest_client_state_from_file()] Try to import client state \
                from serialization file");
            let buffer = read(client_state_file_name.as_str()).expect(
                format!("[fn read_latest_client_state_from_file()] Error while \
                    opening client state file '{}'", client_state_file_name).as_str());
            log::debug!("[fn get_latest_client_state_from_file()] buffer len: {}", buffer.len());
            Ok(buffer)
        } else {
            log::info!("[fn read_latest_client_state_from_file()] Path '{}' does not exist. \
            Returning empty client state.", client_state_file_name);
            Ok(Vec::<u8>::new())
        }
    }

    /// Flushes and frees all read/write buffers and handles
    /// Use prepare() before this function is used.
    pub fn flush_resources(&mut self) -> Result<()>
    {
        self.is_prepared = false;
        if let Some(vfs_fat_handle) = &mut self.vfs_fat_handle {
            log::debug!("[fn flush_resources()] vfs_fat_handle.drop_filesystem()");
            vfs_fat_handle.drop_filesystem()?;
        } else {
            log::debug!("[fn flush_resources()] self.vfs_fat_handle is None");
        }
        Ok(())
    }

    pub fn get_vfs_fat_base_path(&self) -> Result<String> {
        if let Some(vfs_fat_h) = &self.vfs_fat_handle {
            Ok(vfs_fat_h.base_path.clone())
        } else {
            bail!("[fn get_vfs_fat_base_path] No vfs_fat_handle available ")
        }
    }

    pub fn get_client_data_file_name(&self) -> Result<String> {
        if let Some(vfs_fat_h) = &self.vfs_fat_handle {
            Ok(Self::compose_client_state_file_name(vfs_fat_h))
        } else {
            bail!("[fn get_client_data_file_name] No vfs_fat_handle available")
        }
    }

    fn is_client_data_file_path_existing(&self) -> Result<bool> {
        let client_data_file_name = self.get_client_data_file_name()?;
        log::debug!("[fn is_client_data_file_path_existing()] client_data_file_name: '{}'", client_data_file_name);
        let new_path = Path::new(client_data_file_name.as_str());
        log::debug!("[fn is_client_data_file_path_existing()] new_path: '{}'", new_path.display());
        let path_extists = new_path.exists();
        log::debug!("[fn is_client_data_file_path_existing()] path_extists: '{}'", path_extists);
        Ok(path_extists)
    }

    fn write_client_state_to_file(&mut self, client_state: Vec<u8>) -> Result<usize> {
        if let Some(vfs_fat_h) = &self.vfs_fat_handle {
            let file_name = Self::compose_client_state_file_name(vfs_fat_h);
            write(file_name, &client_state)?;
            Ok(client_state.len())
        } else {
            bail!("[fn write_client_state_to_file()] self.vfs_fat_handle is None");
        }
    }

    fn remove_client_state_file_and_clear_latest_state(&mut self) -> Result<()> {
        if let Some(vfs_fat_h) = &self.vfs_fat_handle {
            let file_name = Self::compose_client_state_file_name(vfs_fat_h);
            if Path::new(file_name.as_str()).exists(){
                log::info!("[fn remove_client_state_file_and_clear_latest_state()] Removing file {}", file_name);
                remove_file(file_name)?;
                self.latest_client_data_bytes = Some(Vec::<u8>::new());
            } else {
                log::info!("[fn remove_client_state_file_and_clear_latest_state()] Can not remove file {} cause it does not exist.", file_name);
            }
            Ok(())
        } else {
            bail!("[fn remove_client_state_file_and_clear_latest_state()] self.vfs_fat_handle is None");
        }
    }

    #[allow(unused_assignments)]
    fn persist_client_state_via_callback(&mut self, client_state: Vec<u8>) -> Result<usize> {
        let mut ret_val: usize = 0;
        if let Some(call_back_fn) = self.options.update_client_data_call_back {
            log::info!("[fn persist_client_state_via_callback()] Calling call_back_fn with {} \
                            bytes of data:", client_state.len());
            let success = call_back_fn(
                client_state.as_ptr(),
                client_state.len(),
                self.options.p_update_call_back_caller_user_data,
            );
            if success {
                ret_val = client_state.len();
                self.latest_client_data_bytes = Some(client_state);
            } else {
                log::error!("[fn persist_client_state_via_callback()] call_back_fn returned \
                            false. This streams channel can not be used to send further messages \
                             anymore. Client state data are:\n{:02X?}", client_state);
                bail!("call_back_fn returned false. This streams channel can not be used to send \
                        further messages anymore.");
            }
        } else {
            bail!("self.options.update_client_data_call_back is None");
        }
        Ok(ret_val)
    }

    fn persist_cleared_client_state_via_callback_and_clear_latest_state(&mut self) -> Result<()> {
        let cleared_client_state = Vec::<u8>::new();
        let store_bytes_size = self.persist_client_state_via_callback(cleared_client_state)?;
        if store_bytes_size == 0 {
            Ok(())
        } else {
            bail!("store_bytes_size != 0")
        }
    }
}

impl SubscriberPersistence for ClientDataPersistence {
    fn is_client_state_existing(&self) -> Result<bool> {
        assert!(self.is_prepared);
        match self.options.streams_client_data_storage_type {
            StreamsClientDataStorageType::CLIENT_DATA_STORAGE_VFS_FAT => {
                return self.is_client_data_file_path_existing();
            }
            StreamsClientDataStorageType::CLIENT_DATA_STORAGE_CALL_BACK => {
                if let Some(data) = &self.latest_client_data_bytes {
                    let client_state_exist = data.len() > 0;
                    return Ok(client_state_exist);
                }
            }
        }
        Ok(false)
    }

    fn get_latest_client_state(&self) -> Result<Vec<u8>> {
        assert!(self.is_prepared);
        if let Some(client_data) = &self.latest_client_data_bytes {
            Ok(client_data.clone())
        } else {
            bail!("[fn get_latest_client_state] self.latest_client_data_bytes is None")
        }
    }

    fn persist_new_client_state(&mut self, client_state: Vec<u8>) -> Result<usize> {
        assert!(self.is_prepared);
        match self.options.streams_client_data_storage_type {
            StreamsClientDataStorageType::CLIENT_DATA_STORAGE_VFS_FAT => {
                self.write_client_state_to_file(client_state)
            },
            StreamsClientDataStorageType::CLIENT_DATA_STORAGE_CALL_BACK => {
                self.persist_client_state_via_callback(client_state)
            }
        }
    }

    fn clear_client_state(&mut self) -> Result<()> {
        assert!(self.is_prepared);
        match self.options.streams_client_data_storage_type {
            StreamsClientDataStorageType::CLIENT_DATA_STORAGE_VFS_FAT => {
                self.remove_client_state_file_and_clear_latest_state()
            },
            StreamsClientDataStorageType::CLIENT_DATA_STORAGE_CALL_BACK => {
                self.persist_cleared_client_state_via_callback_and_clear_latest_state()
            }
        }
    }
}

pub async fn create_subscriber<TransportT, WalletT>(
    transport_opt: Option<TransportT::Options>,
    client_data_persistence: Rc<RefCell<ClientDataPersistence>>
) -> Result<SubscriberManager<TransportT, WalletT>>
    where
        TransportT: StreamsTransport,
        WalletT: SimpleWallet
{
    let mut transport = TransportT::new(transport_opt);

    log::debug!("[fn create_subscriber()] Creating Wallet");
    let wallet_path = client_data_persistence.borrow().get_vfs_fat_base_path()? + "/wallet_sensor.txt";
    log::debug!("[fn create_subscriber()] wallet_path: '{}'", wallet_path);
    let wallet = WalletT::new(wallet_path.as_str());
    transport.set_initialization_cnt(wallet.get_initialization_cnt());

    log::debug!("[fn create_subscriber()] Creating subscriber");
    let subscriber= SubscriberManager::<TransportT, WalletT>::new(
        transport,
        wallet,
        client_data_persistence,
    ).await;

    log::debug!("[fn create_subscriber()] Returning created subscriber");
    Ok(subscriber)
}