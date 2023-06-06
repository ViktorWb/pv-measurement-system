use esp_idf_hal::delay::FreeRtos;

/// Uses rusts `panic_hook` to catch a panic and write the error message to the TTGO display.
/// After a timeout, the original panic hook (pani)
pub fn setup_panic_hook(
    display: &'static crate::display::Display<impl embedded_hal_0_2::blocking::i2c::Write + Send>,
) {
    let orig_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let message = if let Some(message) = panic_info.message() {
            format!("{} ", message)
        } else if let Some(payload) = panic_info.payload().downcast_ref::<&'static str>() {
            format!("{}, ", payload)
        } else {
            "Unknown panic message".to_string()
        };

        println!("Oups! A fatal error occurred. Printing to display and waiting 10 seconds before restarting device - {}", panic_info);

        if let Some(location) = panic_info.location() {
            display.push(format!(
                "FATAL ERROR ({}:{}:{}):",
                location.file(),
                location.line(),
                location.column(),
            ));
        } else {
            display.push("FATAL ERROR:".to_string());
        }

        display.push(message);
        FreeRtos::delay_ms(10000);
        display.push("Restarting device".to_string());
        FreeRtos::delay_ms(3000);
        orig_hook(panic_info)
    }));
}
