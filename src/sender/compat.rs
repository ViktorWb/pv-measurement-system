#![allow(deprecated)]

pub struct OldInputOutputPin<T> {
    pin: T,
}

impl<T, E> From<T> for OldInputOutputPin<T>
where
    T: embedded_hal::digital::InputPin<Error = E>,
    E: core::fmt::Debug,
{
    fn from(pin: T) -> Self {
        OldInputOutputPin { pin }
    }
}

impl<T, E> embedded_hal_0_2::digital::InputPin for OldInputOutputPin<T>
where
    T: embedded_hal::digital::InputPin<Error = E>,
    E: core::fmt::Debug,
{
    fn is_low(&self) -> bool {
        self.pin.is_low().unwrap()
    }

    fn is_high(&self) -> bool {
        self.pin.is_high().unwrap()
    }
}

/// Implementation of `v1::OutputPin` trait for fallible `v2::OutputPin` output pins
/// where errors will panic.
#[allow(deprecated)]
impl<T, E> embedded_hal_0_2::digital::OutputPin for OldInputOutputPin<T>
where
    T: embedded_hal::digital::OutputPin<Error = E>,
    E: core::fmt::Debug,
{
    fn set_low(&mut self) {
        self.pin.set_low().unwrap()
    }

    fn set_high(&mut self) {
        self.pin.set_high().unwrap()
    }
}
