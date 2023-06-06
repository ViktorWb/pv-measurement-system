#![allow(deprecated)]

use std::sync::{Arc, Mutex};

use embedded_hal_0_2::{
    digital::{InputPin, OutputPin},
    prelude::_embedded_hal_blocking_delay_DelayMs,
};
use esp_idf_hal::delay::FreeRtos;
use onewire::{DeviceSearch, OneWire};

pub struct Temperature {
    sensor: Arc<Mutex<(OneWire<'static, ()>, onewire::DS18B20)>>,
}

impl Temperature {
    pub fn new(pin: &'static mut (impl OutputPin + InputPin + Send)) -> Self {
        let mut bus = OneWire::new(pin, false);

        bus.reset(&mut FreeRtos).unwrap();

        let mut search_state = DeviceSearch::new();
        let sensor = loop {
            if let Some(device_address) = bus.search_next(&mut search_state, &mut FreeRtos).unwrap()
            {
                println!(
                    "Found device with family code {}",
                    device_address.family_code()
                );
                if device_address.family_code() != onewire::ds18b20::FAMILY_CODE {
                    // skip other devices
                    continue;
                }
                break onewire::DS18B20::new::<()>(device_address).unwrap();
            } else {
                panic!("Failed to find ds18b20 temperature sensor")
            }
        };

        Self {
            sensor: Arc::new(Mutex::new((bus, sensor))),
        }
    }

    pub async fn measure(&self) -> u16 {
        let sensor = self.sensor.clone();
        //smol::unblock(move || {
        let (bus, sensor) = &mut *sensor.lock().unwrap();

        // request sensor to measure temperature
        let resolution = sensor.measure_temperature(bus, &mut FreeRtos).unwrap();

        // wait for compeletion, depends on resolution
        FreeRtos.delay_ms(resolution.time_ms());

        // read temperature
        sensor.read_temperature(bus, &mut FreeRtos).unwrap()
    }
}
