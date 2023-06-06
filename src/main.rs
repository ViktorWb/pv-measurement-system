#![feature(panic_info_message)]
#![feature(future_join)]
#![allow(unused)]

mod display;
mod encryption;
mod lora;
mod panic_hook;
#[cfg(feature = "receiver")]
mod receiver;
#[cfg(feature = "sender")]
mod sender;

use esp_idf_svc::nvs::{EspDefaultNvsPartition, EspNvs, NvsDefault};
use esp_idf_sys as _; // If using the `binstart` feature of `esp-idf-sys`, always keep this module imported

use esp_idf_hal::gpio::{Gpio18, PinDriver};
use esp_idf_hal::i2c::{I2cConfig, I2cDriver};
use esp_idf_hal::prelude::FromValueType;
use esp_idf_hal::{
    prelude::*,
    spi::{Dma, SpiDeviceDriver},
};
use std::sync::{Arc, Mutex};

lazy_static::lazy_static! {
    static ref STORAGE: Mutex<EspNvs<NvsDefault>> = {
        let partition = EspDefaultNvsPartition::take().unwrap();
        Mutex::new(
            esp_idf_svc::nvs::EspNvs::new(partition, "f", true).unwrap(),
        )
    };
}

async fn do_main() -> ! {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_sys::link_patches();
    // This registration is required for delays/timers to function
    esp_idf_sys::esp!(unsafe {
        esp_idf_sys::esp_vfs_eventfd_register(&esp_idf_sys::esp_vfs_eventfd_config_t {
            max_fds: 50,
        })
    })
    .unwrap();
    // Enable backtrace (improved output for panic)
    std::env::set_var("RUST_BACKTRACE", "1");
    // Increase stack size of spawned threads
    std::env::set_var("RUST_MIN_STACK", "7000");

    println!("Hello, world!");

    let peripherals = Peripherals::take().unwrap();

    // SETUP DISPLAY -------------------------------------------------------------------------------------------------

    let i2c = peripherals.i2c0;
    let sda = peripherals.pins.gpio4;
    let scl = peripherals.pins.gpio15;

    let config = I2cConfig::new().baudrate(400.kHz().into());
    let i2c = I2cDriver::new(i2c, sda, scl, &config).unwrap();

    let display: &'static _ = Box::leak(Box::new(display::Display::new(i2c)));
    panic_hook::setup_panic_hook(display);

    // SETUP LORA -------------------------------------------------------------------------------------------------

    let lora_spi = peripherals.spi2;
    let lora_sclk = peripherals.pins.gpio5;
    let lora_mosi = peripherals.pins.gpio19;
    let lora_miso = peripherals.pins.gpio27;

    let lora_reset = PinDriver::output(peripherals.pins.gpio12).unwrap();
    let lora_cs = PinDriver::output(peripherals.pins.gpio18).unwrap();

    let lora_spi_config = esp_idf_hal::spi::config::Config::default().baudrate(200.kHz().into());

    let lora_spi_device = SpiDeviceDriver::new_single(
        lora_spi,
        lora_sclk,
        lora_miso,
        Some(lora_mosi),
        Dma::Disabled,
        None::<Gpio18>,
        &lora_spi_config,
    )
    .unwrap();

    let lora = lora::Lora::new_abp(
        lora_spi_device,
        lora_cs.into_output().unwrap(),
        lora_reset.into_input_output().unwrap(),
        868,
        esp_idf_hal::delay::FreeRtos,
    );
    let lora: &'static _ = Box::leak(Box::new(lora));

    #[cfg(feature = "sender")]
    let sender_fut = sender::run_sender(
        display,
        lora,
        peripherals.i2c1,
        peripherals.pins.gpio21,
        peripherals.pins.gpio22,
        peripherals.ledc,
        peripherals.pins.gpio17,
        peripherals.pins.gpio25,
    );

    #[cfg(feature = "receiver")]
    let receiver_fut = receiver::run_receiver(display, lora, peripherals.modem);

    #[cfg(all(feature = "sender", feature = "receiver"))]
    {
        return smol::future::zip(sender_fut, receiver_fut).await.0;
    }

    #[cfg(all(feature = "sender", not(feature = "receiver")))]
    {
        return sender_fut.await;
    }

    #[cfg(all(feature = "receiver", not(feature = "sender")))]
    {
        return receiver_fut.await;
    }

    #[cfg(not(any(feature = "sender", feature = "receiver")))]
    {
        const _: () = {
            panic!(
                r#"Run with "cargo build --features sender" and/or "cargo build --features receiver" to specify sender or receiver."#
            )
        };
        unreachable!()
    }
}

fn main() {
    smol::block_on(do_main());
}
