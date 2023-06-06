use std::{
    net::Ipv4Addr,
    sync::{Arc, Mutex},
};

use embedded_svc::{
    http::{client::Client, Status},
    io::{Read, Write},
    storage::RawStorage,
    utils::io::try_read_full,
};
use esp_idf_hal::delay::FreeRtos;
use esp_idf_svc::{
    errors::EspIOError,
    http::client::{Configuration, EspHttpConnection},
};
use esp_idf_sys::EspError;

// const AUTHORZATION: &str = "Token JhDxHK7xv3jJKQEQhmenbVb5-a2BcV1Oy8Fhgo5x4uai5XeMWeN34LhALuiccfYmFeGJUO3H9Iw3Yy_wRcZO5w==";

#[derive(Default, Debug)]
pub struct Config {
    pub host: String,
    pub port: u16,
    pub org: String,
    pub bucket: String,
    pub auth: String,
}

enum InfluxWriteError {
    Http(u16),
    Esp(EspIOError),
}

impl std::fmt::Debug for InfluxWriteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Esp(val) => std::fmt::Debug::fmt(val, f),
            Self::Http(val) => write!(f, "Got HTTP status code {val}"),
        }
    }
}

impl From<EspIOError> for InfluxWriteError {
    fn from(value: EspIOError) -> Self {
        Self::Esp(value)
    }
}

impl From<EspError> for InfluxWriteError {
    fn from(value: EspError) -> Self {
        Self::Esp(value.into())
    }
}

fn do_write(config: &Config, body: &str) -> Result<(), InfluxWriteError> {
    println!("Writing to InfluxDB bucket {}:\n{body}", config.bucket);

    let uri = format!(
        "http://{}:{}/api/v2/write?org={}&bucket={}",
        config.host, config.port, config.org, config.bucket
    );
    let body_len_str = body.as_bytes().len().to_string();

    let bearer_str = format!("Token {}", config.auth);

    let headers = [
        ("Authorization", bearer_str.as_str()),
        ("Content-Type", "text/plain; charset=utf-8"),
        ("Accept", "application/json"),
        ("Content-Length", &body_len_str),
    ];

    let mut client = Client::wrap(EspHttpConnection::new(&Configuration {
        crt_bundle_attach: Some(esp_idf_sys::esp_crt_bundle_attach),
        ..Default::default()
    })?);

    let mut req = client.post(&uri, &headers)?;
    req.write_all(body.as_bytes())?;
    req.flush()?;

    let mut response = req.submit()?;

    if response.status() != 204 {
        println!(
            "Failed to write to InfluxDB - Expected HTTP status 204, got {}. Status message: {:?}",
            response.status(),
            response.status_message()
        );

        let mut body = [0_u8; 2048];

        let read = try_read_full(&mut response, &mut body)
            .map_err(|err| err.0)
            .unwrap();

        println!(
            "Body response of last InfluxDB request:\n{:?}",
            String::from_utf8_lossy(&body[..read]).into_owned()
        );

        // Complete the response
        while response.read(&mut body).unwrap() > 0 {}

        return Err(InfluxWriteError::Http(response.status()));
    }

    println!("InfluxDB write success.");
    Ok(())
}

fn set_stored_config(config: &Config) {
    let mut storage_locked = crate::STORAGE.lock().unwrap();
    storage_locked
        .set_raw("DBHOST", config.host.as_bytes())
        .unwrap();
    storage_locked
        .set_raw("DBPORT", &config.port.to_be_bytes())
        .unwrap();
    storage_locked
        .set_raw("DBORG", config.org.as_bytes())
        .unwrap();
    storage_locked
        .set_raw("DBAUTH", config.auth.as_bytes())
        .unwrap();
    storage_locked
        .set_raw("DBBUCKET", config.bucket.as_bytes())
        .unwrap();
}

pub struct Influx {
    write_tx: smol::channel::Sender<String>,
    config_tx: smol::channel::Sender<Config>,
}

impl Influx {
    fn do_get_stored_config() -> Option<Config> {
        let mut storage_locked = crate::STORAGE.lock().unwrap();
        let mut host_target = vec![0; 100];
        let host = storage_locked
            .get_raw("DBHOST", &mut host_target)
            .unwrap()
            .map(|x| String::from_utf8(x.to_vec()));
        let mut port_target = vec![0; 2];
        let port = storage_locked
            .get_raw("DBPORT", &mut port_target)
            .unwrap()
            .map(|x| u16::from_be_bytes(x.try_into().unwrap()));
        let mut org_target = vec![0; 100];
        let org = storage_locked
            .get_raw("DBORG", &mut org_target)
            .unwrap()
            .map(|x| String::from_utf8(x.to_vec()));
        let mut auth_target = vec![0; 100];
        let auth = storage_locked
            .get_raw("DBAUTH", &mut auth_target)
            .unwrap()
            .map(|x| String::from_utf8(x.to_vec()));
        let mut bucket_target = vec![0; 100];
        let bucket = storage_locked
            .get_raw("DBBUCKET", &mut bucket_target)
            .unwrap()
            .map(|x| String::from_utf8(x.to_vec()));
        match (host, port, org, auth, bucket) {
            (Some(Ok(host)), Some(port), Some(Ok(org)), Some(Ok(auth)), Some(Ok(bucket))) => {
                println!("Found Influx host {host}, port {port}, org {org}, auth {auth} and bucket {bucket} in storage");
                Some(Config {
                    host,
                    port,
                    org,
                    auth,
                    bucket,
                })
            }
            _ => None,
        }
    }

    pub fn get_stored_config(&self) -> Option<Config> {
        Influx::do_get_stored_config()
    }

    pub fn new(
        display: &'static crate::display::Display<
            impl embedded_hal_0_2::blocking::i2c::Write + Send,
        >,
    ) -> Self {
        let (write_tx, write_rx) = smol::channel::unbounded::<String>();

        let (config_tx, config_rx) = smol::channel::unbounded::<Config>();
        if let Some(stored_config) = Influx::do_get_stored_config() {
            config_tx.try_send(stored_config).unwrap();
        }

        std::thread::Builder::new()
            .stack_size(10000)
            .spawn(move || {
                let mut last_print = std::time::SystemTime::UNIX_EPOCH;
                let mut last_print_ok = false;

                let mut config = config_rx.recv_blocking().unwrap();
                while let Ok(body) = write_rx.recv_blocking() {
                    let now = std::time::SystemTime::now();
                    loop {
                        while let Ok(new_config) = config_rx.try_recv() {
                            config = new_config;
                        }
                        match do_write(&config, &body) {
                            Ok(_) => {
                                let now = std::time::SystemTime::now();
                                if !last_print_ok || now.duration_since(last_print).unwrap().as_secs() > 180 {
                                    display.push("InfluxDB write success".to_string());
                                    last_print = now;
                                    last_print_ok = true;
                                }
                                break;
                            },
                            Err(e) => {
                                let now = std::time::SystemTime::now();
                                if last_print_ok || now.duration_since(last_print).unwrap().as_secs() > 180 {
                                    display.push(format!("Failed to write to InfluxDB: {e:?}"));
                                    last_print = now;
                                    last_print_ok = false;
                                }
                                match e {
                                    InfluxWriteError::Esp(_) => {
                                        println!("Failed to write to InfluxDB, trying again in 1 second: {e:?}");
                                        FreeRtos::delay_ms(1000);
                                        while let Ok(new_config) = config_rx.try_recv() {
                                            config = new_config;
                                        }
                                    },
                                    InfluxWriteError::Http(_) => {
                                        println!("Failed to write to InfluxDB, will not try again: {e:?}");
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
            })
            .unwrap();

        Self {
            write_tx,
            config_tx,
        }
    }

    pub fn write(&self, body: String) {
        self.write_tx.try_send(body).unwrap();
    }

    pub fn try_write_now(&self, body: String) {
        if let Some(config) = self.get_stored_config() {
            smol::unblock(move || do_write(&config, &body));
        }
    }

    pub fn configure(&self, config: Config) {
        let fixed = Config {
            host: config.host.trim().to_owned(),
            port: config.port,
            org: config.org.trim().to_owned(),
            bucket: config.bucket.trim().to_owned(),
            auth: config.auth.trim().to_owned(),
        };
        set_stored_config(&fixed);
        self.config_tx.try_send(fixed).unwrap();
    }
}