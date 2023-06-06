mod calibration;
mod influx;
mod messages;
mod server;
mod time;
mod wifi;

use embedded_hal_0_2::blocking::delay::DelayMs;
use embedded_hal_0_2::blocking::spi::{Transfer, Write};
use embedded_hal_0_2::digital::v2::OutputPin;
use esp_idf_hal::delay::FreeRtos;
use esp_idf_hal::modem::Modem;
use esp_idf_svc::eventloop::EspSystemEventLoop;
use sntp_request::SntpRequest;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub async fn run_receiver<
    I2C: embedded_hal_0_2::blocking::i2c::Write + Send,
    SPI,
    CS,
    E,
    RESET,
    DELAY,
>(
    display: &'static crate::display::Display<I2C>,
    lora: &'static crate::lora::Lora<SPI, CS, RESET, DELAY>,
    modem: Modem,
) -> !
where
    E: std::fmt::Debug,
    SPI: Transfer<u8, Error = E> + Write<u8, Error = E> + Send + 'static,
    CS: OutputPin + Send + 'static,
    CS::Error: std::fmt::Debug,
    RESET: OutputPin + Send + 'static,
    RESET::Error: std::fmt::Debug,
    DELAY: DelayMs<u8> + Send + 'static,
{
    // SETUP WIFI

    let sysloop = EspSystemEventLoop::take().unwrap();

    let wifi = smol::future::race(
        smol::unblock(|| wifi::WiFi::new(modem, sysloop, display)),
        async {
            smol::Timer::after(std::time::Duration::from_secs(120)).await;
            panic!("After 120 seconds, Wi-Fi was still not connected");
        },
    )
    .await;
    let wifi: &'static _ = Box::leak(Box::new(wifi));

    let influx = Box::leak(Box::new(influx::Influx::new(display)));

    let voltage_calibration = Box::leak(Box::new(calibration::Calibration::new()));
    let current_calibration = Box::leak(Box::new(calibration::Calibration::new()));

    wifi.set_ssid_and_pass("uwe_at_home".to_string(), "bef7DIAF".to_string());
    influx.configure(influx::Config {
        host: "192.168.3.140".to_string(),
        port: 8086,
        org: "angstromlab".to_string(),
        auth: "nKiG9TvFBHvyi2J0_ge9JjZTu-322ermASXzErFwJDJ7hxgnX33l7701Hdxs_bX2NZKcxwWJ1mYbSHD6ozWMaA==".to_string(),
        bucket: "rooftop".to_string(),
    });

    server::start_server(
        server::ServerConfigurations {
            wifi,
            influx,
            voltage_calibration,
            current_calibration,
        },
        display,
    );

    smol::future::zip(
        messages::run_message_receiver(
            display,
            lora,
            influx,
            voltage_calibration,
            current_calibration,
        ),
        async {
            influx.try_write_now(format!(""));
            smol::Timer::after(Duration::from_secs(60)).await;
        },
    )
    .await
    .0;
}
