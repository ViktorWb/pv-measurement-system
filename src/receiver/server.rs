use embedded_svc::{
    http::{
        server::{HandlerError, HandlerResult, Request},
        Headers, Method,
    },
    io::{adapters::ToStd, Error, Read, Write},
};
use esp_idf_svc::http::server::{EspHttpConnection, EspHttpServer};
use std::{
    collections::{HashMap, HashSet},
    net::Ipv4Addr,
    str::FromStr,
};

pub struct ServerConfigurations {
    pub wifi: &'static super::wifi::WiFi,
    pub influx: &'static super::influx::Influx,
    pub current_calibration: &'static super::calibration::Calibration,
    pub voltage_calibration: &'static super::calibration::Calibration,
}

pub fn start_server(
    configs: ServerConfigurations,
    display: &crate::display::Display<impl embedded_hal_0_2::blocking::i2c::Write>,
) {
    let mut server = EspHttpServer::new(&Default::default()).unwrap();

    server
        .fn_handler("/", Method::Get, |req| {
            let (ssid, pass) = configs.wifi.get_stored_ssid_and_pass().unwrap_or_default();
            let connection_status = if configs.wifi.is_connected() {
                "Connected"
            } else {
                "Disconnected"
            };
            let super::influx::Config {
                host: influx_host,
                port: influx_port,
                org: influx_org,
                auth: influx_auth,
                bucket: influx_bucket,
            } = configs.influx.get_stored_config().unwrap_or_default();
            req.into_ok_response()?.write_all(
                format!(
                    r#"<doctype html5>
<html>

<body style="font-family: Helvetica, sans-serif; margin: 1em;">
    <form method="post" action="/setwifi" enctype="application/x-www-form-urlencoded"
        style="display: inline-block; border: 1px solid lightgrey; padding: 0 1em 1em 1em;">
        <h2>Configure Wi-Fi:</h2>
        <div style="display: grid; grid-template-columns: auto 500px; gap: 0.5em 2em;">
            WiFi SSID:
            <input name="ssid" type="text" value="{ssid}">
            WiFi password:
            <input name="pass" type="text" value="">
        </div>
        <input type="submit" value="Set" style="margin: 0.5em 0 0 auto; display: block;">
    </form>
    <div style="display: inline-block; border: 1px solid lightgrey; padding: 0 1em 1em 1em; vertical-align: top;">
        <h4>Connection status:</h4>
        {connection_status}
    </div>
    <br />
    <form method="post" action="/setinflux" enctype="application/x-www-form-urlencoded"
        style="display: inline-block; border: 1px solid lightgrey; padding: 0 1em 1em 1em;">
        <h2>Configure InfluxDB:</h2>
        Enter all values before pressing "Set"
        <br />
        <br />
        <div style="display: grid; grid-template-columns: auto 500px; gap: 0.5em 2em;">
            Host/IP, for example: 192.168.1.1 or mydomain.com:
            <input name="host" type="text" value="{influx_host}">
            TCP port, for example 8086:
            <input name="port" type="text" value="{influx_port}">
            Org:
            <input name="org" type="text" value="{influx_org}">
            Auth token:
            <input name="auth" type="text" value="{influx_auth}">
            Bucket:
            <input name="bucket" type="text" value="{influx_bucket}">
        </div>
        <br />
        <input type="submit" value="Set" style="margin: 0.5em 0 0 auto; display: block;">
    </form>
    <br />
    <form method="post" action="/setvoltagecalibration" enctype="application/x-www-form-urlencoded"
        style="display: inline-block; border: 1px solid lightgrey; padding: 0 1em 1em 1em;">
        <h2>Set voltage calibration:</h2>
        <span style="display: block; width: 500px;">
            Enter the ID for the device to calibrate, then enter the calibration points. In the left field, enter
            the actually applied current, and in the right field, enter the value which was measured by the device
            at this current (without any calibration). Enter as many points as you'd like.
        </span>
        <br />
        <span style="display: block; width: 500px;">
            To remove calibration, enter the device ID and leave all other fields empty.
        </span>
        <br />
        <br />
        <div style="display: grid; grid-template-columns: auto auto; gap: 0.5em 2em;">
            Device ID:
            <input name="devid" type="text" value="">
            <span style="grid-column: 1/3;">Enter calibration points below:</span>
            <input name="val1" type="text" value="">
            <input name="val2" type="text" value="">
            <input name="val3" type="text" value="">
            <input name="val4" type="text" value="">
            <input name="val5" type="text" value="">
            <input name="val6" type="text" value="">
            <input name="val7" type="text" value="">
            <input name="val8" type="text" value="">
            <input name="val9" type="text" value="">
            <input name="val10" type="text" value="">
            <input name="val11" type="text" value="">
            <input name="val12" type="text" value="">
            <input name="val13" type="text" value="">
            <input name="val14" type="text" value="">
            <input name="val15" type="text" value="">
            <input name="val16" type="text" value="">
            <input name="val17" type="text" value="">
            <input name="val18" type="text" value="">
            <input name="val19" type="text" value="">
            <input name="val20" type="text" value="">
            <input name="val21" type="text" value="">
            <input name="val22" type="text" value="">
        </div>
        <br />
        <input type="submit" value="Set" style="margin: 0.5em 0 0 auto; display: block;">
    </form>
    <br />
    <form method="post" action="/setcurrentcalibration" enctype="application/x-www-form-urlencoded"
        style="display: inline-block; border: 1px solid lightgrey; padding: 0 1em 1em 1em;">
        <h2>Set current calibration:</h2>
        <span style="display: block; width: 500px;">
            Enter the ID for the device to calibrate, then enter the calibration points. In the left field, enter
            the actually applied current, and in the right field, enter the value which was measured by the device
            at this current (without any calibration). Enter as many points as you'd like.
        </span>
        <br />
        <span style="display: block; width: 500px;">
            To remove calibration, enter the device ID and leave all other fields empty.
        </span>
        <br />
        <br />
        <div style="display: grid; grid-template-columns: auto auto; gap: 0.5em 2em;">
            Device ID:
            <input name="devid" type="text" value="">
            <span style="grid-column: 1/3;">Enter calibration points below:</span>
            <input name="val1" type="text" value="">
            <input name="val2" type="text" value="">
            <input name="val3" type="text" value="">
            <input name="val4" type="text" value="">
            <input name="val5" type="text" value="">
            <input name="val6" type="text" value="">
            <input name="val7" type="text" value="">
            <input name="val8" type="text" value="">
            <input name="val9" type="text" value="">
            <input name="val10" type="text" value="">
            <input name="val11" type="text" value="">
            <input name="val12" type="text" value="">
            <input name="val13" type="text" value="">
            <input name="val14" type="text" value="">
            <input name="val15" type="text" value="">
            <input name="val16" type="text" value="">
            <input name="val17" type="text" value="">
            <input name="val18" type="text" value="">
            <input name="val19" type="text" value="">
            <input name="val20" type="text" value="">
            <input name="val21" type="text" value="">
            <input name="val22" type="text" value="">
        </div>
        <br />
        <input type="submit" value="Set" style="margin: 0.5em 0 0 auto; display: block;">
    </form>
</body>

</html>"#
                )
                .as_bytes(),
            )?;

            Ok(())
        })
        .unwrap();
    server
        .fn_handler("/setwifi", Method::Post, |mut req| {
            let Some(Ok(length)) = req.header("Content-Length").map(|x| x.parse::<usize>()) else {
                return HandlerResult::Err(HandlerError::new("Invalid header Content-Length"));
            };

            let mut body = vec![0; length];
            if req.read_exact(&mut body).is_err() {
                return Err(HandlerError::new("Failed to read body"));
            }

            let parsed = url::form_urlencoded::parse(&body);
            let mut params = parsed
                .clone()
                .filter(|p| p.0 == "ssid" || p.0 == "pass")
                .collect::<HashMap<_, _>>();

            let ssid = params
                .remove("ssid")
                .ok_or(HandlerError::new("Missing parameter ssid"))?;
            let pass = params
                .remove("pass")
                .ok_or(HandlerError::new("Missing parameter pass"))?;

            println!("Setting Wi-Fi to SSID {ssid} and password {pass}");

            configs
                .wifi
                .set_ssid_and_pass(ssid.to_string(), pass.to_string());

            Ok(())
        })
        .unwrap();

    server
        .fn_handler("/setinflux", Method::Post, |mut req| {
            let Some(Ok(length)) = req.header("Content-Length").map(|x| x.parse::<usize>()) else {
                return HandlerResult::Err(HandlerError::new("Invalid header Content-Length"));
            };

            let mut body = vec![0; length];
            if req.read_exact(&mut body).is_err() {
                return Err(HandlerError::new("Failed to read body"));
            }

            let parsed = url::form_urlencoded::parse(&body);
            let mut params = parsed
                .clone()
                .filter(|p| {
                    p.0 == "host"
                        || p.0 == "port"
                        || p.0 == "org"
                        || p.0 == "auth"
                        || p.0 == "bucket"
                })
                .collect::<HashMap<_, _>>();

            let host = params
                .remove("host")
                .ok_or(HandlerError::new("Missing parameter host"))?
                .to_string();
            let port = params
                .remove("port")
                .ok_or(HandlerError::new("Missing parameter port"))?
                .parse()
                .map_err(|_| HandlerError::new("Invalid port, expected 16-bit number"))?;
            let org = params
                .remove("org")
                .ok_or(HandlerError::new("Missing parameter org"))?
                .to_string();
            let auth = params
                .remove("auth")
                .ok_or(HandlerError::new("Missing parameter auth"))?
                .to_string();
            let bucket = params
                .remove("bucket")
                .ok_or(HandlerError::new("Missing parameter bucket"))?
                .to_string();

            let config = super::influx::Config {
                host,
                port,
                org,
                auth,
                bucket,
            };

            println!("Setting InfluxDB config to {:?}", config);

            configs.influx.configure(config);

            Ok(())
        })
        .unwrap();

    let set_calibration = |voltage: bool, mut req: Request<&mut EspHttpConnection>| {
        let Some(Ok(length)) = req.header("Content-Length").map(|x| x.parse::<usize>()) else {
            return HandlerResult::Err(HandlerError::new("Invalid header Content-Length"));
        };

        let mut body = vec![0; length];
        if req.read_exact(&mut body).is_err() {
            return Err(HandlerError::new("Failed to read body"));
        }

        let value_names: Vec<String> = (1..23).map(|i| format!("val{i}")).collect();

        let parsed = url::form_urlencoded::parse(&body);
        let mut params = parsed
            .clone()
            .filter(|p| p.0 == "devid" || value_names.contains(&p.0.to_string()))
            .collect::<HashMap<_, _>>();

        let device_id = params
            .remove("devid")
            .ok_or(HandlerError::new("Missing parameter devid"))?
            .parse()
            .map_err(|_| HandlerError::new("Failed to parse device id as 8-bit unsigned int"))?;

        let values = value_names
            .iter()
            .map(|name| {
                let Some(value) = params.get(name.as_str()) else {
                    return Err(HandlerError::new(&format!("Missing parameter {name}")));
                };
                let value = value.trim();
                if value == "" {
                    return Ok(None);
                }
                let Ok(parsed) = value.parse() else {
                    return Err(HandlerError::new(&format!("Failed to parse {value} as float 32")));
                };
                Ok(Some(parsed))
            })
            .collect::<Result<Vec<Option<f32>>, _>>()?
            .chunks_exact(2)
            .filter_map(|list| match (list[0], list[1]) {
                (Some(a), Some(b)) => Some(super::calibration::CalibrationPoint {
                    actual: a,
                    measured: b,
                }),
                _ => None,
            })
            .collect::<Vec<_>>();

        println!(
            "Setting current calibration configuration for device {device_id}: {:?}",
            values
        );

        if voltage {
            configs
                .voltage_calibration
                .set_calibration(device_id, values);
        } else {
            configs
                .current_calibration
                .set_calibration(device_id, values);
        }

        Ok(())
    };

    server
        .fn_handler("/setvoltagecalibration", Method::Post, move |mut req| {
            set_calibration(true, req)
        })
        .unwrap();

    server
        .fn_handler("/setcurrentcalibration", Method::Post, move |mut req| {
            set_calibration(false, req)
        })
        .unwrap();

    display.push(format!(
        r#"Configure at: SSID: ttgo, pass: ttgolora2023, ip: {}"#,
        configs.wifi.get_ip_on_access_point()
    ));

    Box::leak(Box::new(server));
}
