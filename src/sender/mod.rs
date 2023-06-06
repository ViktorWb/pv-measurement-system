mod compat;
mod mppt;
mod temperature;

use embedded_hal_0_2::adc::OneShot;
use embedded_hal_0_2::blocking::delay::DelayMs;
use embedded_hal_0_2::blocking::spi::{Transfer, Write};
use embedded_hal_0_2::digital::v1_compat::{OldInputPin, OldOutputPin};
use embedded_hal_0_2::digital::v2::OutputPin;
use esp_idf_hal::delay::FreeRtos;
use esp_idf_hal::gpio::{Gpio13, Gpio17, Gpio21, Gpio22, Gpio23, Gpio25, Gpio4, PinDriver};
use esp_idf_hal::i2c::{I2cConfig, I2cDriver, I2C1};
use esp_idf_hal::ledc::{config::TimerConfig, LEDC};
use esp_idf_hal::ledc::{LedcDriver, LedcTimerDriver};
use esp_idf_hal::prelude::FromValueType;
use std::cell::RefCell;
use std::time::Duration;

pub async fn run_sender<
    I2C: embedded_hal_0_2::blocking::i2c::Write + Send,
    SPI,
    CS,
    E,
    RESET,
    DELAY,
>(
    display: &'static crate::display::Display<I2C>,
    lora: &'static crate::lora::Lora<SPI, CS, RESET, DELAY>,
    i2c: I2C1,
    sda: Gpio21,
    scl: Gpio22,
    ledc: LEDC,
    timer: Gpio17,
    temperature_pin: Gpio25,
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
    let config = I2cConfig::new().baudrate(400.kHz().into());
    let i2c = I2cDriver::new(i2c, sda, scl, &config).unwrap();

    let address = ads1x1x::SlaveAddr::default();
    let mut adc = ads1x1x::Ads1x1x::new_ads1115(i2c, address);
    adc.set_full_scale_range(ads1x1x::FullScaleRange::Within1_024V)
        .unwrap();

    let adc = RefCell::new(adc);

    let channel = LedcDriver::new(
        ledc.channel0,
        LedcTimerDriver::new(
            ledc.timer0,
            &TimerConfig::new()
                .frequency(50.kHz().into())
                .resolution(esp_idf_hal::ledc::Resolution::Bits10),
        )
        .unwrap(),
        timer,
    )
    .unwrap();

    let measure_voltage = || async {
        nb::block!(adc.borrow_mut().read(&mut ads1x1x::channel::SingleA0))
            .unwrap()
            .max(0) as u16
    };

    let measure_current = || async {
        nb::block!(adc.borrow_mut().read(&mut ads1x1x::channel::SingleA1))
            .unwrap()
            .max(0) as u16
    };

    /*
    // Unfortunately, the sensor DS18B20 is never found. Unsure why.
    let temperature = temperature::Temperature::new(Box::leak(Box::new(
        compat::OldInputOutputPin::from(PinDriver::input_output_od(temperature_pin).unwrap()),
    )));
    */

    let mut mppt = mppt::Mppt::new(
        measure_voltage().await,
        measure_current().await,
        channel,
        &mut FreeRtos,
    );

    const SENDER_ID: u8 = {
        let val = konst::result::unwrap_ctx!(konst::primitive::parse_u8(std::env!("DEVICE_ID")));
        assert!(val < 2u8.pow(7));
        val
    };

    struct MeasurementPoint {
        voltage: u16,
        current: u16,
    }

    let send_mppt = |points: Vec<MeasurementPoint>, start_time: std::time::SystemTime| async move {
        const MPPT_DESTINATION: u8 = 0;

        let first_byte = SENDER_ID << 1 | MPPT_DESTINATION;

        let mut message = Vec::with_capacity(1 + points.len() * 4 + 2);
        message.push(first_byte);

        for point in points {
            message.extend_from_slice(&point.voltage.to_be_bytes());
            message.extend_from_slice(&point.current.to_be_bytes());
        }

        let total_duration = std::time::SystemTime::now()
            .duration_since(start_time)
            .unwrap()
            .as_millis() as u32;
        println!("Total duration: {total_duration}");
        let duration_per_point = (total_duration / ((message.len() as u32 - 1) / 4)) as u16;
        println!("Duration per point: {duration_per_point}");
        message.extend_from_slice(&duration_per_point.to_be_bytes());

        println!("Sending: {message:?}");
        let to_send = super::encryption::encrypt(&message);
        display.push(format!("Sending mppt message of {} bytes", to_send.len()));
        println!("Sending encrypted message: {:?}", to_send);
        lora.send_raw_message(&to_send).await.unwrap();
    };

    let send_sweep = |points: Vec<MeasurementPoint>| async {
        const SWEEP_DESTINATION: u8 = 1;

        let first_byte = SENDER_ID << 1 | SWEEP_DESTINATION;

        let mut message = Vec::with_capacity(1 + points.len() * 4);
        message.push(first_byte);

        for point in points {
            message.extend_from_slice(&point.voltage.to_be_bytes());
            message.extend_from_slice(&point.current.to_be_bytes());
        }

        println!("Sending: {message:?}");
        let to_send = super::encryption::encrypt(&message);
        display.push(format!("Sending sweep message of {} bytes", to_send.len()));
        println!("Sending encrypted message: {:?}", to_send);
        lora.send_raw_message(&to_send).await.unwrap();
    };

    let mut count: u64 = 0;
    let mut start_time = std::time::SystemTime::now();
    let mut mppt_points = Vec::with_capacity(25);

    loop {
        if count % 6000 == 0 {
            display.push("Sweep".to_owned());

            let num_points = 40;
            let mut sweep_points = Vec::with_capacity(num_points);

            struct MaxPoint {
                duty: f32,
                voltage: u16,
                current: u16,
                power: u32,
            }

            let mut max_power = MaxPoint {
                duty: 0.0,
                voltage: 0,
                current: 0,
                power: 0,
            };

            for x in 0..num_points {
                let duty = x as f32 / num_points as f32;

                mppt.set_pwm(duty);

                smol::Timer::after(Duration::from_millis(50)).await;

                let voltage = measure_voltage().await;
                let current = measure_current().await;

                let power = voltage as u32 * current as u32;
                if power > max_power.power {
                    max_power = MaxPoint {
                        duty,
                        voltage,
                        current,
                        power,
                    };
                }

                sweep_points.push(MeasurementPoint { voltage, current });
            }

            if max_power.current < 300 {
                if !mppt_points.is_empty() {
                    send_mppt(mppt_points, start_time).await;
                } else {
                    send_mppt(
                        vec![MeasurementPoint {
                            voltage: max_power.voltage,
                            current: max_power.current,
                        }],
                        start_time,
                    )
                    .await;
                }

                unsafe { esp_idf_sys::esp_deep_sleep(60 * 1_000_000) };
            }

            send_sweep(sweep_points).await;

            mppt.set_operating_point(max_power.duty);
        }

        // After a quick measurement of timing 70 ms delay gave 100 ms total MPPT iteration duration
        smol::Timer::after(Duration::from_millis(70)).await;

        println!("\nRunning MPPT iteration");
        let voltage = measure_voltage().await;
        let current = measure_current().await;
        mppt.iteration(voltage, current);

        if count % 100 == 0 {
            display.push(format!("voltage: {voltage}, current: {current}"));
        }

        count += 1;
        println!("Count: {count}");
        if count % 100 == 0 {
            mppt_points.push(MeasurementPoint { voltage, current });
            if mppt_points.len() >= 25 {
                send_mppt(mppt_points, start_time).await;

                start_time = std::time::SystemTime::now();
                mppt_points = vec![];
            }
        }
    }
}
