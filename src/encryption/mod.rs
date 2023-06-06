use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};

#[cfg(feature = "sender")]
mod nonce;

const KEY: &[u8; 32] = {
    // Edit the following encryption key before compiling. Note that it must be 32 
    let my_key: &[u8; 32] = &[
        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24,
        25, 26, 27, 28, 29, 30, 31,
    ];

    if konst::const_eq!(
        my_key,
        &[
            0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23,
            24, 25, 26, 27, 28, 29, 30, 31
        ]
    ) {
        panic!("Before compiling, edit the 256-bit encryption key");
    }
    my_key
};

lazy_static::lazy_static! {
    static ref CIPHER: Aes256Gcm = {
        Aes256Gcm::new(generic_array::GenericArray::from_slice(KEY))
    };
}

#[cfg(feature = "sender")]
pub fn encrypt(message: &[u8]) -> Vec<u8> {
    let nonce = nonce::get_and_increment_nonce();

    let mut nonce_bytes = [0; 12];
    nonce_bytes[10..].copy_from_slice(&nonce.to_be_bytes());

    let encrypted = CIPHER
        .encrypt(Nonce::from_slice(&nonce_bytes), message)
        .unwrap();
    [nonce.to_be_bytes().to_vec(), encrypted].concat()
}

pub fn decrypt(message: &[u8]) -> Option<(u16, Vec<u8>)> {
    if message.len() < 2 {
        return None;
    }

    let nonce = u16::from_be_bytes(message[0..2].try_into().unwrap());

    let mut nonce_bytes = [0; 12];
    nonce_bytes[10..].copy_from_slice(&nonce.to_be_bytes());

    CIPHER
        .decrypt(Nonce::from_slice(&nonce_bytes), &message[2..])
        .ok()
        .map(|decrypted| (nonce, decrypted))
}
