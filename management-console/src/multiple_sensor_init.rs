use async_trait::async_trait;

use log;

use anyhow::{
    anyhow,
    Result as AnyResult
};

use streams_tools::{
    explorer::{
        error::AppError,
        threading_helpers::{
            Worker,
            run_worker_in_own_thread,
        }
    },
    multi_channel_management::{
        get_initial_channel_manager,
        MultiChannelManagerOptions
    },
    remote::remote_sensor::{
        RemoteSensor,
        RemoteSensorOptions
    },
    UserDataStore
};

use crate::{
    cli::ManagementConsoleCli,
    create_channel,
    create_remote_sensor_options,
    get_multi_channel_manager_options,
    make_remote_sensor_register_keyload_msg,
    subscribe_remote_sensor_to_channel
};

pub(crate) async fn init_sensor_in_own_thread<'a>(user_store: &UserDataStore, cli: &ManagementConsoleCli<'a>, dev_eui: String) -> AnyResult<()> {
    let init_sensor_opt = InitSensorOptions::new(
        user_store,
        cli,
        dev_eui,
    )?;

    tokio::spawn(async move {
        run_worker_in_own_thread::<InitSensor>(init_sensor_opt).await.map_err(|app_err| anyhow!("{:?}", app_err))
    });

    Ok(())
}

#[derive(Clone)]
struct InitSensorOptions {
    user_store: UserDataStore,
    mult_chan_mngr_opt: MultiChannelManagerOptions,
    remote_sensor_options: RemoteSensorOptions,
    dev_eui: String,
}

impl InitSensorOptions {
    pub fn new<'a>(user_store: &UserDataStore, cli: &ManagementConsoleCli<'a>, dev_eui: String) -> AnyResult<InitSensorOptions> {
        let mult_chan_mngr_opt = get_multi_channel_manager_options(cli)?;
        let remote_sensor_options = create_remote_sensor_options(cli, Some(dev_eui.clone()));
        Ok(InitSensorOptions{
            user_store: user_store.clone(),
            mult_chan_mngr_opt,
            remote_sensor_options,
            dev_eui,
        })
    }
}

struct InitSensor;

#[async_trait(?Send)]
impl Worker for InitSensor {
    type OptionsType = InitSensorOptions;
    type ResultType = ();

    async fn run(opt: InitSensorOptions) -> Result<(), AppError> {
        log::info!("DevEUI: {} - Starting initialization thread", opt.dev_eui);
        let mut channel_manager  = get_initial_channel_manager(
            &opt.user_store,
            &opt.mult_chan_mngr_opt
        ).await?;

        log::debug!("DevEUI: {} - channel_manager is ready - calling create_channel()", opt.dev_eui);
        let announcement_link = create_channel(&mut channel_manager).await
            .expect("Could not create_channel");
        let remote_sensor = RemoteSensor::new(Some(opt.remote_sensor_options));
        log::debug!("DevEUI: {} - remote_sensor is ready - calling subscribe_remote_sensor_to_channel()", opt.dev_eui);
        let subscription = subscribe_remote_sensor_to_channel(&remote_sensor, announcement_link).await?;
        log::debug!("DevEUI: {} - calling make_remote_sensor_register_keyload_msg()", opt.dev_eui);
        let _keyload_registration = make_remote_sensor_register_keyload_msg(&mut channel_manager, &remote_sensor, subscription).await?;
        Ok(())
    }
}