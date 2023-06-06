use std::net::Ipv4Addr;
use std::num::NonZeroI32;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use embedded_svc::storage::RawStorage;
use embedded_svc::wifi::{AccessPointConfiguration, AuthMethod, Wifi};
use embedded_svc::wifi::{ClientConfiguration, Configuration};
use esp_idf_hal::delay::FreeRtos;
use esp_idf_hal::peripheral;
use esp_idf_svc::ping::EspPing;
use esp_idf_svc::wifi::WifiWait;
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    netif::{EspNetif, EspNetifWait},
    wifi::EspWifi,
};
use esp_idf_sys::EspError;
use sntp_request::SntpRequest;

lazy_static::lazy_static! {
    static ref ACCESS_POINT_CONFIG: AccessPointConfiguration = AccessPointConfiguration {
        ssid: "ttgo".into(),
        password: "ttgolora2023".into(),
        auth_method: AuthMethod::WPA2Personal,
        ..Default::default()
    };
}

fn ping(ip: Ipv4Addr) -> Result<(), ()> {
    let ping_summary = EspPing::default()
        .ping(ip, &Default::default())
        .map_err(|_| ())?;
    if ping_summary.transmitted != ping_summary.received {
        println!("Pinging IP {} resulted in timeouts", ip);
        return Err(());
    }
    println!("Pinging done");
    Ok(())
}

fn try_connect(
    display: &'static crate::display::Display<impl embedded_hal_0_2::blocking::i2c::Write + Send>,
    wifi: &mut EspWifi,
    sysloop: &EspSystemEventLoop,
    ssid: &str,
    pass: &str,
) -> Result<(), Option<EspError>> {
    display.push(format!(r#"Searching for Wi-Fi "{ssid}""#));

    let scan_result = wifi.scan()?;
    println!("Found Wifi networks: {:?}", scan_result);
    let ours = scan_result.into_iter().find(|a| a.ssid == ssid);
    let channel = if let Some(ours) = ours {
        display.push(format!(r#"Found network. Connecting"#));
        println!(
            "Found configured access point {} on channel {}",
            ssid, ours.channel
        );
        Some(ours.channel)
    } else {
        display.push(format!(r#"Scan did not find network."#));
        println!(
            "Configured access point {} not found during scanning, will go with unknown channel",
            ssid
        );
        None
    };

    wifi.set_configuration(&Configuration::Mixed(
        ClientConfiguration {
            ssid: ssid.into(),
            password: pass.into(),
            channel,
            ..Default::default()
        },
        ACCESS_POINT_CONFIG.clone(),
    ))?;

    wifi.start()?;
    println!("Starting wifi...");
    if !WifiWait::new(&sysloop)?
        .wait_with_timeout(Duration::from_secs(20), || wifi.is_started() == Ok(true))
    {
        println!("Wifi did not start in 20 seconds");
        return Err(None);
    }

    wifi.connect()?;

    if !EspNetifWait::new::<EspNetif>(wifi.sta_netif(), &sysloop)?.wait_with_timeout(
        Duration::from_secs(20),
        || {
            if wifi.is_connected() != Ok(true) {
                return false;
            }
            let Ok(ip) = wifi.sta_netif().get_ip_info().map(|x| x.ip) else {
                return false;
            };
            ip != Ipv4Addr::new(0, 0, 0, 0)
        },
    ) {
        println!("Wifi did not connect or did not receive a DHCP lease");
        return Err(None);
    }

    display.push("Wi-Fi connected!".to_string());

    Ok(())
}

pub struct WiFi {
    set_wifi_tx: smol::channel::Sender<(String, String)>,
    is_connected: Arc<Mutex<bool>>,
    ip_on_access_point: Ipv4Addr,
}

impl WiFi {
    pub fn set_ssid_and_pass(&self, ssid: String, pass: String) {
        let mut storage_locked = crate::STORAGE.lock().unwrap();
        println!("Writing Wi-Fi credentials {ssid} and {pass} to storage");
        storage_locked.set_raw("WIFISSID", ssid.as_bytes()).unwrap();
        storage_locked.set_raw("WIFIPASS", pass.as_bytes()).unwrap();
        drop(storage_locked);
        self.set_wifi_tx.try_send((ssid, pass)).unwrap();
    }

    fn do_get_stored_ssid_and_pass() -> Option<(String, String)> {
        let mut storage_locked = crate::STORAGE.lock().unwrap();
        let mut ssid_target = vec![0; 100];
        let ssid = storage_locked
            .get_raw("WIFISSID", &mut ssid_target)
            .unwrap()
            .map(|x| String::from_utf8(x.to_vec()));
        let mut pass_target = vec![0; 100];
        let pass = storage_locked
            .get_raw("WIFIPASS", &mut pass_target)
            .unwrap()
            .map(|x| String::from_utf8(x.to_vec()));
        match (ssid, pass) {
            (Some(Ok(ssid)), Some(Ok(pass))) => {
                println!("Found Wi-Fi credentials {ssid} and {pass} in storage");
                Some((ssid, pass))
            }
            _ => None,
        }
    }

    pub fn get_stored_ssid_and_pass(&self) -> Option<(String, String)> {
        WiFi::do_get_stored_ssid_and_pass()
    }

    pub fn is_connected(&self) -> bool {
        *self.is_connected.lock().unwrap()
    }

    pub fn get_ip_on_access_point(&self) -> Ipv4Addr {
        self.ip_on_access_point
    }

    pub fn new(
        modem: impl peripheral::Peripheral<P = esp_idf_hal::modem::Modem> + Send + 'static,
        sysloop: EspSystemEventLoop,
        display: &'static crate::display::Display<
            impl embedded_hal_0_2::blocking::i2c::Write + Send,
        >,
    ) -> Self {
        let (set_wifi_tx, set_wifi_rx) = smol::channel::unbounded();
        if let Some((ssid, pass)) = WiFi::do_get_stored_ssid_and_pass() {
            set_wifi_tx.try_send((ssid, pass)).unwrap();
        } else {
            display.push("Wifi not configured.".to_string());
        }

        let access_point_config = AccessPointConfiguration {
            ssid: "ttgo".into(),
            password: "ttgolora2023".into(),
            auth_method: AuthMethod::WPA2Personal,
            ..Default::default()
        };

        let mut wifi = Box::leak(Box::new(
            EspWifi::new(modem, sysloop.clone(), None).unwrap(),
        ));

        wifi.set_configuration(&Configuration::AccessPoint(access_point_config.clone()))
            .unwrap();

        wifi.start().unwrap();
        println!("Starting wifi...");
        if !WifiWait::new(&sysloop)
            .unwrap()
            .wait_with_timeout(Duration::from_secs(20), || wifi.is_started() == Ok(true))
        {
            panic!("Wifi did not start in 20 seconds");
        }

        let ip_on_access_point = wifi.ap_netif().get_ip_info().unwrap().ip;

        let is_connected = Arc::new(Mutex::new(false));
        let is_connected_clone = Arc::clone(&is_connected);

        std::thread::spawn(move || {
            let mut prev_ssid: Option<String> = None;
            let mut prev_pass: Option<String> = None;

            loop {
                let mut count: u16 = 0;
                let (mut ssid, mut pass) = loop {
                    count = count + 1;
                    if let Ok((ssid, pass)) = set_wifi_rx.try_recv() {
                        break (ssid, pass);
                    }
                    FreeRtos::delay_ms(500);
                    if count & 10 == 0 {
                        let is_connected = wifi.sta_netif().is_up() == Ok(true);
                        println!("WiFi is connected: {is_connected}");
                        let mut is_connected_locked = is_connected_clone.lock().unwrap();
                        *is_connected_locked = is_connected;
                        drop(is_connected_locked);
                    }
                    if count % 240 == 0 {
                        // Every 120 seconds, try to connect if it isn't already connected
                        if let (Some(prev_ssid), Some(prev_pass)) = (&prev_ssid, &prev_pass) {
                            break (prev_ssid.clone(), prev_pass.clone());
                        }
                    }
                };
                while let Ok((ssid2, pass2)) = set_wifi_rx.try_recv() {
                    ssid = ssid2;
                    pass = pass2;
                }

                let is_connected = wifi.sta_netif().is_up() == Ok(true);
                println!("WiFi is connected: {is_connected}");
                let mut is_connected_locked = is_connected_clone.lock().unwrap();
                *is_connected_locked = is_connected;
                drop(is_connected_locked);

                if is_connected
                    && prev_ssid.as_ref() == Some(&ssid)
                    && prev_pass.as_ref() == Some(&pass)
                {
                    continue;
                }

                try_connect(display, &mut wifi, &sysloop, &ssid, &pass);

                prev_ssid = Some(ssid);
                prev_pass = Some(pass);
            }
        });

        Self {
            set_wifi_tx,
            is_connected,
            ip_on_access_point,
        }
    }
}
