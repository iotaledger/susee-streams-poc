use anyhow::{
    Result,
    bail,
};

use embedded_svc::{
    wifi::{
        Configuration,
        ClientConfiguration,
        AccessPointConfiguration,
    }
};

use esp_idf_svc::{
    wifi::{
        EspWifi,
        BlockingWifi,
    },
    eventloop::{
        EspSystemEventLoop
    },
    nvs::{
        EspDefaultNvsPartition,
    },
    ping,
};

use esp_idf_hal::{
    peripherals::Peripherals,
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

    let mut esp_wifi = Box::<EspWifi>::new(EspWifi::new(
        peripherals.modem,
        sys_loop.clone(),
        Some(default_nvs_part)
    )?);
    let mut wifi = BlockingWifi::wrap(esp_wifi.as_mut(), sys_loop)?;

    log::info!("Wifi created, setting default configuration");
    wifi.set_configuration(&Configuration::Client(ClientConfiguration::default()))?;

    log::info!("Going to start wifi");
    wifi.start()?;

    log::info!("Wifi started, about to scan available networks");
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

    log::info!("Wifi configuration for prefered network set, going to connect");
    wifi.connect()?;

    log::info!("Waiting for Wifi netif up ...");
    wifi.wait_netif_up()?;
    log::info!("Wifi netif is up");

    let ip_info = esp_wifi.sta_netif().get_ip_info()?;

    log::info!("Wifi DHCP info: {:?}", ip_info);

    ping(&ip_info.subnet.gateway)?;

    Ok(esp_wifi)
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