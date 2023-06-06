use esp_idf_hal::ledc::LedcDriver;

mod ic;
mod po;

const INITIAL_GUESS: f32 = 0.2;

pub struct Mppt<'a> {
    ledc_driver: LedcDriver<'a>,

    voltage_old: i32,
    current_old: i32,
    output: f32,
}

impl<'a> Mppt<'a> {
    pub fn new(
        initial_voltage: u16,
        initial_current: u16,
        ledc_driver: LedcDriver<'a>,
        delay: &mut impl embedded_hal_0_2::blocking::delay::DelayMs<u16>,
    ) -> Self {
        let voltage_old = initial_voltage as i32;
        let current_old = initial_current as i32;

        let mut mppt = Self {
            ledc_driver,

            voltage_old,
            current_old,
            output: 0.0,
        };

        let output = INITIAL_GUESS;
        mppt.output = output;

        mppt.set_pwm(output);
        mppt
    }

    pub fn set_operating_point(&mut self, mut duty: f32) {
        duty = duty.min(1.0).max(0.0);
        self.output = duty;
        self.set_pwm(duty);
    }

    pub fn set_pwm(&mut self, mut duty: f32) {
        duty = duty.min(1.0).max(0.0);
        self.ledc_driver
            .set_duty((self.ledc_driver.get_max_duty() as f32 * duty) as u32)
            .unwrap();
    }

    pub fn iteration(&mut self, voltage: u16, current: u16) {
        println!("Voltage new: {voltage}");
        println!("Current new: {current}");

        self.output = po::iteration(
            self.output,
            voltage.into(),
            current.into(),
            self.voltage_old,
            self.current_old,
        );

        println!("Output: {}", self.output);

        self.set_pwm(self.output);

        self.voltage_old = voltage.into();
        self.current_old = current.into();
    }

    pub fn sweep(&mut self, delay: &mut impl embedded_hal_0_2::blocking::delay::DelayMs<u16>) {
        let end = 10;
        let values: Vec<_> = (0..end)
            .map(|x| {
                let duty = x as f32 / end as f32;

                self.set_pwm(duty);

                delay.delay_ms(500);

                (0, 0)
                //((self.read_voltage)(), (self.read_current)())
            })
            .collect();
        println!("{:?}", values);
    }
}
