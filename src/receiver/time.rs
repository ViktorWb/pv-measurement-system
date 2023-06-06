use std::time::Duration;

use sntp_request::SntpRequest;

/// Get the current time as number of seconds since Jan 1, 1970
pub async fn get_current_time() -> i64 {
    println!("Fetching time from network");
    let sntp = SntpRequest::new();
    loop {
        match sntp.get_unix_time() {
            Ok(val) => {
                println!("Got time {val} from network");
                return val;
            },
            Err(e) => match e.kind() {
                std::io::ErrorKind::WouldBlock => continue,
                _ => {
                    println!("Failed to fetch time from network");
                    smol::Timer::after(Duration::from_secs(1)).await;
                    continue;
                }
            },
        }
    }
}
