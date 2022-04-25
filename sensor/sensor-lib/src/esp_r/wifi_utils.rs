use embedded_svc::{
    ipv4,
    ping::Ping,
    wifi::{
        Wifi,
        Configuration,
        ClientConfiguration,
        AccessPointConfiguration,
        Status,
        ClientStatus,
        ClientConnectionStatus,
        ClientIpStatus,
        ApStatus,
        ApIpStatus,
    }
};

use esp_idf_svc::{
    wifi::{
        EspWifi,
    },
    netif::{
        EspNetifStack,
    },
    sysloop::{
        EspSysLoopStack
    },
    nvs::{
        EspDefaultNvs,
    },
    ping,
};

use std::{
    sync::Arc,
    time::Duration
};

use anyhow::{
    Result,
    bail,
};

const SSID: &str = env!("SENSOR_MAIN_POC_WIFI_SSID");
const PASS: &str = env!("SENSOR_MAIN_POC_WIFI_PASS");

// *************************************************************************************************
// *                                                                                               *
// *    Wifi utility functions taken from                                                          *
// *    https://github.com/ivmarkov/rust-esp32-std-demo/blob/main/src/main.rs                      *
// *                                                                                               *
// *************************************************************************************************

pub fn init_wifi() -> Result<(Box<EspWifi>, ipv4::ClientSettings)> {
    let netif_stack = Arc::new(EspNetifStack::new()?);
    let sys_loop_stack = Arc::new(EspSysLoopStack::new()?);
    let default_nvs = Arc::new(EspDefaultNvs::new()?);

    let mut wifi = Box::new(EspWifi::new(netif_stack, sys_loop_stack, default_nvs)?);

    println!("Wifi created, about to scan");

    let ap_infos = wifi.scan()?;

    let ours = ap_infos.into_iter().find(|a| a.ssid == SSID);

    let channel = if let Some(ours) = ours {
        println!(
            "Found configured access point {} on channel {}",
            SSID, ours.channel
        );
        Some(ours.channel)
    } else {
        println!(
            "Configured access point {} not found during scanning, will go with unknown channel",
            SSID
        );
        None
    };

    wifi.set_configuration(&Configuration::Mixed(
        ClientConfiguration {
            ssid: SSID.into(),
            password: PASS.into(),
            channel,
            ..Default::default()
        },
        AccessPointConfiguration {
            ssid: "aptest".into(),
            channel: channel.unwrap_or(1),
            ..Default::default()
        },
    ))?;

    println!("Wifi configuration set, about to get status");

    wifi.wait_status_with_timeout(Duration::from_secs(20), |status| !status.is_transitional())
        .map_err(|e| anyhow::anyhow!("Unexpected Wifi status: {:?}", e))?;

    let status = wifi.get_status();

    let client_settings: ipv4::ClientSettings;
    if let Status(
        ClientStatus::Started(ClientConnectionStatus::Connected(ClientIpStatus::Done(ip_settings))),
        ApStatus::Started(ApIpStatus::Done),
    ) = status
    {
        println!("Wifi connected");
        client_settings = ip_settings;
    } else {
        bail!("Unexpected Wifi status: {:?}", status);
    }

    Ok((wifi, client_settings))
}

pub fn ping(ip_settings: &ipv4::ClientSettings) -> Result<()> {
    println!("About to do some pings for {:?}", ip_settings);

    let ping_summary =
        ping::EspPing::default().ping(ip_settings.subnet.gateway, &Default::default())?;
    if ping_summary.transmitted != ping_summary.received {
        bail!(
            "Pinging gateway {} resulted in timeouts",
            ip_settings.subnet.gateway
        );
    }

    println!("Pinging done");

    Ok(())
}