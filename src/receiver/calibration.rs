use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

#[derive(Debug)]
pub struct CalibrationPoint {
    pub actual: f32,
    pub measured: f32,
}

pub struct Calibration {
    calibrations: Arc<Mutex<HashMap<u8, Vec<CalibrationPoint>>>>,
}

impl Calibration {
    pub fn new() -> Self {
        Self {
            calibrations: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn calibrate(&self, device_id: u8, value: f32) -> f32 {
        let locked = self.calibrations.lock().unwrap();
        let Some(calibrations) = locked.get(&device_id) else {
            return value;
        };

        for (i, point) in calibrations.iter().enumerate() {
            if point.measured >= value {
                let calibration_factor = point.actual / point.measured;
                return value * calibration_factor;
            }
            if let Some(next) = calibrations.get(i + 1) {
                if next.measured > value {
                    let ratio = (value - point.measured) / (next.measured - point.measured);
                    return point.actual + ratio * (next.actual - point.actual);
                }
            }
        }

        if let Some(last) = calibrations.last() {
            let calibration_factor = last.actual / last.measured;
            return value * calibration_factor;
        }

        value
    }

    pub fn set_calibration(&self, device_id: u8, mut calibrations: Vec<CalibrationPoint>) {
        calibrations.sort_by(|a, b| a.measured.partial_cmp(&b.measured).unwrap());
        let mut locked = self.calibrations.lock().unwrap();
        locked.insert(device_id, calibrations);
    }
}

/*fn main() {
    let calibrations = Calibration::new();

    calibrations.set_calibration(
        75,
        vec![
            CalibrationPoint {
                actual: 0.0,
                measured: 0.0,
            },
            CalibrationPoint {
                actual: 1.0,
                measured: 2949 as f32 * 10.0 / 32767.0,
            },
            CalibrationPoint {
                actual: 2.0,
                measured: 6071 as f32 * 10.0 / 32767.0,
            },
            CalibrationPoint {
                actual: 3.0,
                measured: 9203 as f32 * 10.0 / 32767.0,
            },
            CalibrationPoint {
                actual: 4.0,
                measured: 12352 as f32 * 10.0 / 32767.0,
            },
            CalibrationPoint {
                actual: 5.0,
                measured: 15515 as f32 * 10.0 / 32767.0,
            },
            CalibrationPoint {
                actual: 6.0,
                measured: 18714 as f32 * 10.0 / 32767.0,
            },
            CalibrationPoint {
                actual: 7.0,
                measured: 21918 as f32 * 10.0 / 32767.0,
            },
            CalibrationPoint {
                actual: 8.0,
                measured: 25162 as f32 * 10.0 / 32767.0,
            },
            CalibrationPoint {
                actual: 9.0,
                measured: 28426 as f32 * 10.0 / 32767.0,
            },
            CalibrationPoint {
                actual: 10.0,
                measured: 31779 as f32 * 10.0 / 32767.0,
            },
        ],
    );

    println!("{}", calibrations.calibrate(75, 1396 as f32 * 10.0 / 32767.0));
    println!("{}", calibrations.calibrate(75, 4498 as f32 * 10.0 / 32767.0));
    println!("{}", calibrations.calibrate(75, 7633 as f32 * 10.0 / 32767.0));
    println!("{}", calibrations.calibrate(75, 10784 as f32 * 10.0 / 32767.0));
    println!("{}", calibrations.calibrate(75, 13947 as f32 * 10.0 / 32767.0));
    println!("{}", calibrations.calibrate(75, 17118 as f32 * 10.0 / 32767.0));
    println!("{}", calibrations.calibrate(75, 20310 as f32 * 10.0 / 32767.0));
    println!("{}", calibrations.calibrate(75, 23529 as f32 * 10.0 / 32767.0));
    println!("{}", calibrations.calibrate(75, 26783 as f32 * 10.0 / 32767.0));
    println!("{}", calibrations.calibrate(75, 30106 as f32 * 10.0 / 32767.0));
}*/
