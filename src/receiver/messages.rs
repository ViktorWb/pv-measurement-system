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

pub async fn run_message_receiver<
    I2C: embedded_hal_0_2::blocking::i2c::Write + Send,
    SPI,
    CS,
    E,
    RESET,
    DELAY,
>(
    display: &'static crate::display::Display<I2C>,
    lora: &'static crate::lora::Lora<SPI, CS, RESET, DELAY>,
    influx: &'static super::influx::Influx,
    voltage_calibration: &'static super::calibration::Calibration,
    current_calibration: &'static super::calibration::Calibration,
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
    

    let mut received_nonces = HashMap::new();

    loop {
        let start_wait = std::time::SystemTime::now();
        display.push("Waiting for LoRa message..".to_string());
        let msg = smol::future::race(lora.receive_message(), async {
            smol::Timer::after(std::time::Duration::from_secs(60)).await;
            display.push("Waiting for LoRa  (1 minute)..".to_string());
            smol::Timer::after(std::time::Duration::from_secs(540)).await;
            display.push("Waiting for LoRa  (10 minutes)..".to_string());
            let mut minute = 10;
            loop {
                smol::Timer::after(std::time::Duration::from_secs(minute * 60)).await;
                minute += minute;
                display.push(format!("Waiting for LoRa  ({minute} minutes).."));
            }
        })
        .await;
        display.push(format!("Got LoRa message of {} bytes", msg.len()));

        println!("Got encrypted message: {:?}", msg);

        let Some((nonce, decrypted)) = crate::encryption::decrypt(&msg) else {
            display.push("Decryption failed. Skipping message".to_string());
            continue;
        };

        println!("Got decrypted LoRa message: {:?}", decrypted);

        if decrypted.len() < 1 {
            display.push("Message was empty. Skipping.".to_string());
            continue;
        }

        let timestamp = super::time::get_current_time().await;

        if let Some(previous_timestamp) = received_nonces.get(&nonce) {
            if *previous_timestamp > timestamp - 3600 {
                display.push(format!(
                    "Duplicate nonce {nonce} in past hour. Skipping message."
                ));
                continue;
            }
        }
        received_nonces.insert(nonce, timestamp);

        let first_byte = decrypted[0];
        let destination = first_byte & 1;
        let id = first_byte >> 1;

        let voltages_and_currents_bytes = if destination == 0 {
            if decrypted.len() < 3 {
                display.push("Invalid message. Skipping.".to_string());
                continue;
            }
            &decrypted[1..decrypted.len() - 2]
        } else {
            &decrypted[1..]
        };

        if voltages_and_currents_bytes.len() % 4 != 0 {
            display.push("Invalid message. Skipping.".to_string());
            continue;
        }

        let mut voltages_and_currents = voltages_and_currents_bytes
            .chunks(4)
            .map(|chunk| {
                let voltage =
                    u16::from_be_bytes(chunk[0..2].try_into().unwrap()) as f32 / (32768.0 / 100.0);
                let current =
                    u16::from_be_bytes(chunk[2..4].try_into().unwrap()) as f32 / (32768.0 / 10.0);
                (
                    voltage_calibration.calibrate(id, voltage),
                    current_calibration.calibrate(id, current),
                )
            })
            .collect::<Vec<_>>();

        if destination == 0 {
            // MPP point
            let mut timestamp_ms = timestamp * 1000;
            let millis_between =
                u16::from_be_bytes(decrypted[decrypted.len() - 2..].try_into().unwrap()) as i64;

            println!(
                "Writing {} MPP points at time={}",
                voltages_and_currents.len(),
                timestamp_ms
            );

            voltages_and_currents.reverse();
            for (voltage, current) in voltages_and_currents {
                influx.write(format!(
                    "mppt,host=ttgo{} voltage={voltage},current={current} {}",
                    id,
                    timestamp_ms * 1_000_000
                ));
                timestamp_ms -= millis_between;
            }
        } else {
            // Sweep
            let mut timestamp_ms = timestamp * 1000;

            println!(
                "Writing {} MPP points at time={}",
                voltages_and_currents.len(),
                timestamp_ms
            );

            voltages_and_currents.reverse();
            for (voltage, current) in voltages_and_currents {
                influx.write(format!(
                    "sweep,host=ttgo{} voltage={voltage},current={current} {}",
                    id,
                    timestamp_ms * 1_000_000
                ));
                timestamp_ms -= 1;
            }
        }
    }
}
