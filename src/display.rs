use embedded_graphics::{
    mono_font::{MonoTextStyleBuilder, iso_8859_1::FONT_4X6},
    pixelcolor::BinaryColor,
    prelude::*,
    text::{Baseline, Text},
};
use ssd1306::{mode::BufferedGraphicsMode, prelude::*, I2CDisplayInterface, Ssd1306};
use std::sync::Mutex;

const FIT_MESSAGES: usize = 64 / 6;
const FIT_CHARACTERS: usize = 128 / 4;

struct Inner<I2C: embedded_hal_0_2::blocking::i2c::Write> {
    driver: Ssd1306<I2CInterface<I2C>, DisplaySize128x64, BufferedGraphicsMode<DisplaySize128x64>>,
    messages: Vec<String>,
}

impl<I2C: embedded_hal_0_2::blocking::i2c::Write> Inner<I2C> {
    fn redraw(&mut self) {
        self.driver.clear();

        let text_style = MonoTextStyleBuilder::new()
            .font(&FONT_4X6)
            .text_color(BinaryColor::On)
            .build();

        for (i, value) in self.messages.iter().enumerate() {
            let y = i * 6;

            Text::with_baseline(value, Point::new(0, y as i32), text_style, Baseline::Top)
                .draw(&mut self.driver)
                .unwrap();
        }

        self.driver.flush().unwrap();
    }

    fn do_push(&mut self, message: String) {
        if self.messages.len() == FIT_MESSAGES {
            self.messages.remove(0);
        }
        println!("DISPLAY: {message}");
        self.messages.push(message);
    }

    fn push(&mut self, message: String) {
        if message.len() > FIT_CHARACTERS {
            let chars = message.chars().collect::<Vec<_>>();
            for chunk in chars.chunks(FIT_CHARACTERS) {
                self.do_push(chunk.iter().collect::<String>().trim().to_owned());
            }
        } else {
            self.do_push(message);
        }
        self.redraw();
    }
}

pub struct Display<I2C: embedded_hal_0_2::blocking::i2c::Write> {
    inner: Option<Mutex<Inner<I2C>>>,
}

impl<I2C: embedded_hal_0_2::blocking::i2c::Write> Display<I2C> {
    pub fn new(i2c: I2C) -> Self {
        const USE_DISPLAY: bool = {
            const DISPLAY_ENV: &str = std::env!("USE_DISPLAY");
            if !konst::eq_str(DISPLAY_ENV, "true") && !konst::eq_str(DISPLAY_ENV, "false") {
                panic!("Expected environment variable DISPLAY to equal true or false");
            }
            konst::eq_str(DISPLAY_ENV, "true")
        };

        if !USE_DISPLAY {
            Self { inner: None }
        } else {
            let interface = I2CDisplayInterface::new(i2c);
            let mut driver = Ssd1306::new(interface, DisplaySize128x64, DisplayRotation::Rotate0)
                .into_buffered_graphics_mode();
            driver.init().unwrap();

            Self {
                inner: Some(Mutex::new(Inner {
                    driver,
                    messages: Vec::with_capacity(FIT_MESSAGES),
                })),
            }
        }
    }

    pub fn push(&self, message: String) {
        if let Some(inner) = self.inner.as_ref() {
            let mut locked = inner.lock().unwrap();
            locked.push(message);
        }
    }
}
