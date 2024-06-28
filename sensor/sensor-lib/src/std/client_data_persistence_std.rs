use std::{
    path::Path,
    rc::Rc,
    cell::RefCell,
        fs::{
        write,
        read,
        remove_file,
    }
};

use anyhow::{
    Result,
};

use streams_tools::{
    subscriber_manager::SubscriberPersistence,
};

pub struct ClientDataPersistenceStd {
    file_name: String,
}

impl ClientDataPersistenceStd {
    pub fn new(file_name: &str) -> ClientDataPersistenceStd {
        ClientDataPersistenceStd{
            file_name: file_name.to_string(),
        }
    }

    pub fn new_prepared(file_name: &str) -> Rc<RefCell<ClientDataPersistenceStd>> {
        Rc::new(RefCell::new(
            ClientDataPersistenceStd::new(file_name)
        ))
    }
}

impl SubscriberPersistence for ClientDataPersistenceStd {
    fn is_client_state_existing(&self) -> Result<bool> {
        log::debug!("[fn is_client_state_existing()] file_name: '{}'", self.file_name);
        let new_path = Path::new(self.file_name.as_str());
        log::debug!("[fn is_client_state_existing()] new_path: '{}'", new_path.display());
        let path_exists = new_path.exists();
        log::debug!("[fn is_client_state_existing()] path_exists: '{}'", path_exists);
        Ok(path_exists)
    }

    fn get_latest_client_state(&self) -> Result<Vec<u8>> {
        log::debug!("[fn get_latest_client_state()] read file '{}'", self.file_name);
        let buffer = read(self.file_name.as_str()).
            expect(format!("[SubscriberManager::import_from_serialization_file()] Error while \
                opening channel state file '{}'", self.file_name).as_str());
        log::debug!("[fn get_latest_client_state()] buffer len: {}", buffer.len());
        Ok(buffer)
    }

    fn persist_new_client_state(&mut self, client_state: Vec<u8>) -> Result<usize> {
        log::debug!("[fn persist_new_client_state()] write file '{}'", self.file_name);
        write(self.file_name.as_str(), &client_state)
            .expect(format!("[ClientDataPersistenceStd.persist_new_client_state()] Error while \
                    writing client state data to file '{}'", self.file_name).as_str());
        Ok(client_state.len())
    }

    fn clear_client_state(&mut self) -> Result<()> {
        if Path::new(self.file_name.as_str()).exists(){
            log::info!("[fn clear_client_state()] Removing file {}", self.file_name.as_str());
            remove_file(self.file_name.as_str())?;
        } else {
            log::info!("[fn clear_client_state()] Can not remove file {} cause it does not exist.",
                       self.file_name.as_str());
        }
        Ok(())
    }
}