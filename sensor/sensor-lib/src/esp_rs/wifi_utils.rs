use embedded_svc::{
    wifi::{
        Wifi,
        Configuration,
        ClientConfiguration,
        AccessPointConfiguration,
    }
};

use esp_idf_svc::{
    wifi::{
        EspWifi,
        WifiWait,
    },
    eventloop::{
        EspSystemEventLoop
    },
    nvs::{
        EspDefaultNvsPartition,
    },
    ping,
    netif::{
        EspNetifWait,
        EspNetif
    }
};

use esp_idf_hal::{
    peripherals::Peripherals,
};

use std::{
    time::Duration
};

use anyhow::{
    Result,
    bail,
};

use smol::net::Ipv4Addr;

// *************************************************************************************************
// *                                                                                               *
// *    Wifi utility functions taken from                                                          *
// *    https://github.com/ivmarkov/rust-esp32-std-demo/blob/main/src/main.rs                      *
// *                                                                                               *
// *************************************************************************************************

pub fn init_wifi(wifi_ssid: &str, wifi_pass: &str) -> Result<Box<EspWifi<'static>>> {
    let sys_loop = EspSystemEventLoop::take()?;
    let peripherals = Peripherals::take().unwrap();
    let default_nvs_part = EspDefaultNvsPartition::take()?;

    let mut wifi = Box::new(EspWifi::new(
        peripherals.modem,
        sys_loop.clone(),
        Some(default_nvs_part)
    )?);

    log::info!("Wifi created, about to scan");

    let ap_infos = wifi.scan()?;

    let ours = ap_infos.into_iter().find(|a| a.ssid == wifi_ssid);

    let channel = if let Some(ours) = ours {
        log::info!(
            "Found configured access point {} on channel {}",
            wifi_ssid, ours.channel
        );
        Some(ours.channel)
    } else {
        log::info!(
            "Configured access point {} not found during scanning, will go with unknown channel",
            wifi_ssid
        );
        None
    };

    wifi.set_configuration(&Configuration::Mixed(
        ClientConfiguration {
            ssid: wifi_ssid.into(),
            password: wifi_pass.into(),
            channel,
            ..Default::default()
        },
        AccessPointConfiguration {
            ssid: "aptest".into(),
            channel: channel.unwrap_or(1),
            ..Default::default()
        },
    ))?;

    log::info!("Wifi configuration set, about to wifi.start()");
    wifi.start()?;

    if !WifiWait::new(&sys_loop)?
        .wait_with_timeout(Duration::from_secs(20), || wifi.is_started().unwrap())
    {
        bail!("Wifi did not start");
    }

    log::info!("Connecting wifi...");


    wifi.connect()?;

    if !EspNetifWait::new::<EspNetif>(wifi.sta_netif(), &sys_loop)?.wait_with_timeout(
        Duration::from_secs(20),
        || {
            wifi.is_connected().unwrap()
                && wifi.sta_netif().get_ip_info().unwrap().ip != Ipv4Addr::new(0, 0, 0, 0)
        },
    ) {
        bail!("Wifi did not connect or did not receive a DHCP lease");
    }

    let ip_info = wifi.sta_netif().get_ip_info()?;

    log::info!("Wifi DHCP info: {:?}", ip_info);

    ping(&ip_info.subnet.gateway)?;

    Ok(wifi)
}

pub fn ping(ipv4_addr: &Ipv4Addr) -> Result<()> {
    log::info!("About to do some pings for {:?}", ipv4_addr);

    let ping_summary =
        ping::EspPing::default().ping(ipv4_addr.clone(), &Default::default())?;
    if ping_summary.transmitted != ping_summary.received {
        bail!(
            "Pinging gateway {} resulted in timeouts",
            ipv4_addr
        );
    }

    log::info!("Pinging done");

    Ok(())
}