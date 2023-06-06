use std::sync::{Arc, Mutex};

use embedded_svc::storage::RawStorage;

#[derive(Clone)]
pub struct Nonce {
    did_update: Arc<Mutex<Option<u16>>>,
}

impl Nonce {
    pub fn new() -> Self {
        Self {
            did_update: Arc::new(Mutex::new(None)),
        }
    }

    pub fn get_nonce(&self) -> u16 {
        let mut did_update_locked = self.did_update.lock().unwrap();
        if let Some(nonce) = &*did_update_locked {
            return *nonce;
        }

        let mut keystore_locked = crate::STORAGE.lock().unwrap();

        let mut nonce_target = [0; 2];
        keystore_locked.get_raw("nonce", &mut nonce_target).unwrap();

        let nonce = u16::from_be_bytes(nonce_target);

        println!("Nonce is now {}", nonce);

        keystore_locked
            .set_raw("nonce", &(nonce + 1).to_be_bytes())
            .unwrap();

        *did_update_locked = Some(nonce);
        nonce
    }
}
