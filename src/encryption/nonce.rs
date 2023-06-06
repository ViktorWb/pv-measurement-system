use std::sync::{Arc, Mutex};

use embedded_svc::storage::RawStorage;

const NONCE_MIN: u16 =
    konst::result::unwrap_ctx!(konst::primitive::parse_u16(std::env!("NONCE_MIN")));
const NONCE_MAX: u16 =
    konst::result::unwrap_ctx!(konst::primitive::parse_u16(std::env!("NONCE_MAX")));

lazy_static::lazy_static! {
    static ref INITIALIZE: () = {
        let mut keystore_locked = crate::STORAGE.lock().unwrap();

        if !keystore_locked.contains("hasnonce").unwrap() {
            keystore_locked
                .set_raw("nonce", &NONCE_MIN.to_be_bytes())
                .unwrap();
            keystore_locked
                .set_raw("hasnonce", b"true")
                .unwrap();
        }
    };
}

pub fn get_and_increment_nonce() -> u16 {
    let mut keystore_locked = crate::STORAGE.lock().unwrap();

    let mut nonce_target = [0; 2];
    keystore_locked.get_raw("nonce", &mut nonce_target).unwrap();

    let nonce = u16::from_be_bytes(nonce_target);

    println!("Nonce is now {}", nonce);

    if nonce == NONCE_MAX {
        keystore_locked
            .set_raw("nonce", &NONCE_MIN.to_be_bytes())
            .unwrap();
    } else {
        keystore_locked
            .set_raw("nonce", &(nonce + 1).to_be_bytes())
            .unwrap();
    }

    nonce
}
