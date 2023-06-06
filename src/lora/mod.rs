use std::sync::{Arc, Mutex};

use embedded_hal_0_2::blocking::delay::DelayMs;
use embedded_hal_0_2::blocking::spi::{Transfer, Write};
use embedded_hal_0_2::digital::v2::OutputPin;

mod nonce;
mod sx;

// For OTAA
const APPKEY: u128 = 0x0728DC78F6C7FD95A3E2BC4E92EB614B;
const DEVEUI: u64 = 0x70B3D57ED005CD07;
const APPEUI: u64 = 0x0000000000000000; // Also called JoinEUI
                                        // ---
                                        // For ABP:
const APP_SKEY: u128 = 0xC288AA8452516B9D0966247E84297DDF;
const NWK_SKEY: u128 = 0x4C3282BF029E8E3CFF1F60A172D5F917;
const DEV_ADDR: u32 = 0x727493;
// ---

pub struct Lora<SPI, CS, RESET, DELAY> {
    lora: Arc<std::sync::Mutex<sx::LoRa<SPI, CS, RESET, DELAY>>>,
    nonce: nonce::Nonce,
    nwk_skey: lorawan::keys::AES128,
    app_skey: lorawan::keys::AES128,
    dev_addr: lorawan::parser::DevAddr<[u8; 4]>,

    #[cfg(all(feature = "sender", feature = "receiver"))]
    internal_sender: smol::channel::Sender<Vec<u8>>,
    #[cfg(all(feature = "sender", feature = "receiver"))]
    internal_receiver: smol::lock::Mutex<smol::channel::Receiver<Vec<u8>>>,
}

impl<SPI, CS, RESET, DELAY, E> Lora<SPI, CS, RESET, DELAY>
where
    E: std::fmt::Debug,
    SPI: Transfer<u8, Error = E> + Write<u8, Error = E> + Send + 'static,
    CS: OutputPin + Send + 'static,
    CS::Error: std::fmt::Debug,
    RESET: OutputPin + Send + 'static,
    RESET::Error: std::fmt::Debug,
    DELAY: DelayMs<u8> + Send + 'static,
{
    #[cfg(all(feature = "sender", feature = "receiver"))]
    pub async fn receive_message(&self) -> Vec<u8> {
        self.internal_receiver.lock().await.recv().await.unwrap()
    }

    #[cfg(not(all(feature = "sender", feature = "receiver")))]
    pub async fn receive_message(&self) -> Vec<u8> {
        let lora = Arc::clone(&self.lora);
        smol::unblock(move || {
            println!("Now waiting for LoRa message on sx1276");
            let mut lock = lora.lock().unwrap();
            println!("LOCKED");
            let size = lock.poll_irq(None).unwrap();
            lock.read_packet().unwrap()[0..size].to_vec()
        })
        .await
    }

    #[cfg(all(feature = "sender", feature = "receiver"))]
    pub async fn send_raw_message(&self, message: &[u8]) -> Result<(), ()> {
        if message.len() > 255 {
            panic!(
                "Cannot transmit more than 255 bytes of data. Attempted to transmit: {:?}",
                message
            );
        }

        self.internal_sender.send(message.to_vec()).await.unwrap();
        return Ok(());
    }

    #[cfg(not(all(feature = "sender", feature = "receiver")))]
    pub async fn send_raw_message(&self, message: &[u8]) -> Result<(), ()> {
        if message.len() > 255 {
            panic!(
                "Cannot transmit more than 255 bytes of data. Attempted to transmit: {:?}",
                message
            );
        }

        let mut buffer = [0; 255];
        for (i, c) in message.iter().enumerate() {
            buffer[i] = *c;
        }
        let message_len = message.len();

        let lora = Arc::clone(&self.lora);
        smol::unblock(move || {
            println!("Transmitting {} bytes.", message_len);
            let mut lock = lora.lock().unwrap();
            let transmit = lock.transmit_payload(buffer, message_len);
            while lock.transmitting().unwrap() {
                println!("Transmitting..");
            }

            match transmit {
                Ok(_) => {
                    println!("Successfully transmitted {} bytes.", message_len);
                    Ok(())
                }
                Err(e) => {
                    println!("Failed to transmit: {:?}", e);
                    Err(())
                }
            }
        })
        .await
    }

    fn do_otaa(mut lora: sx::LoRa<SPI, CS, RESET, DELAY>, nonce: nonce::Nonce) -> Self {
        let mut phy = lorawan::creator::JoinRequestCreator::new();
        let key = lorawan::keys::AES128(APPKEY.to_be_bytes());
        phy.set_app_eui(&APPEUI.to_be_bytes());
        phy.set_dev_eui(&DEVEUI.to_be_bytes());
        phy.set_dev_nonce(&nonce.get_nonce().to_be_bytes());
        let payload = phy.build(&key).unwrap();

        let mut buffer = [0; 255];
        for (i, c) in payload.iter().enumerate() {
            buffer[i] = *c;
        }
        loop {
            println!("Transmitting JoinRequest.");
            let transmit = lora.transmit_payload(buffer, payload.len());
            match transmit {
                Ok(()) => {
                    println!("JoinRequest successfully transmitted.");
                    break;
                }
                Err(e) => {
                    println!("Failed to transmit JoinRequest: {:?}", e);
                    esp_idf_hal::delay::FreeRtos::delay_ms(1000);
                    println!("Trying again.");
                }
            }
        }

        println!("Waiting indefinetely for JoinAccept.");
        lora.poll_irq(None).unwrap();
        let response = lora.read_packet().unwrap();

        println!("Got LoRa message!");

        let parsed = lorawan::parser::parse(response).unwrap();
        let payload = match parsed {
            lorawan::parser::PhyPayload::JoinAccept(
                lorawan::parser::JoinAcceptPayload::Encrypted(payload),
            ) => payload.decrypt(&key),
            msg => {
                panic!("Expected JoinAccept, got {:?}", msg)
            }
        };

        println!("Got JoinAccept: {:?}", payload);

        let nonce_bytes = nonce.get_nonce().to_be_bytes();
        let dev_nonce: lorawan::parser::DevNonce<_> = (&nonce_bytes).into();
        let nwk_skey = payload.derive_newskey(&dev_nonce, &key);
        let app_skey = payload.derive_appskey(&dev_nonce, &key);
        let dev_addr = payload.dev_addr().to_owned();

        #[cfg(all(feature = "sender", feature = "receiver"))]
        let (internal_sender, internal_receiver) = smol::channel::bounded(1);

        Self {
            lora: Arc::new(Mutex::new(lora)),
            nonce,
            nwk_skey,
            app_skey,
            dev_addr,

            #[cfg(all(feature = "sender", feature = "receiver"))]
            internal_receiver: smol::lock::Mutex::new(internal_receiver),
            #[cfg(all(feature = "sender", feature = "receiver"))]
            internal_sender,
        }
    }

    pub fn do_abp(lora: sx::LoRa<SPI, CS, RESET, DELAY>, nonce: nonce::Nonce) -> Self {
        let dev_addr_bytes = DEV_ADDR.to_be_bytes();
        let dev_addr_brw: lorawan::parser::DevAddr<&[u8; 4]> = (&dev_addr_bytes).into();

        #[cfg(all(feature = "sender", feature = "receiver"))]
        let (internal_sender, internal_receiver) = smol::channel::bounded(1);

        Self {
            lora: Arc::new(Mutex::new(lora)),
            nonce,
            app_skey: APP_SKEY.to_be_bytes().into(),
            nwk_skey: NWK_SKEY.to_be_bytes().into(),
            dev_addr: dev_addr_brw.to_owned(),

            #[cfg(all(feature = "sender", feature = "receiver"))]
            internal_receiver: smol::lock::Mutex::new(internal_receiver),
            #[cfg(all(feature = "sender", feature = "receiver"))]
            internal_sender,
        }
    }

    fn setup_lora(
        spi: SPI,
        cs: CS,
        reset: RESET,
        frequency: i64,
        delay: DELAY,
    ) -> sx::LoRa<SPI, CS, RESET, DELAY> {
        println!("Starting communications with sx1276..");

        let mut lora = sx::LoRa::new(spi, cs, reset, frequency, delay)
            .expect("Failed to communicate with radio module!");

        println!("Communications with sx1276 established!");

        lora.set_coding_rate_4(5).unwrap();
        lora.set_spreading_factor(7).unwrap();
        lora.set_preamble_length(8).unwrap();
        lora.set_signal_bandwidth(125000).unwrap();
        lora.set_tx_power(17, 1).unwrap();
        lora.set_crc(true).unwrap();

        lora
    }

    pub fn new_otaa(spi: SPI, cs: CS, reset: RESET, frequency: i64, delay: DELAY) -> Self {
        Self::do_otaa(
            Self::setup_lora(spi, cs, reset, frequency, delay),
            nonce::Nonce::new(),
        )
    }

    pub fn new_abp(spi: SPI, cs: CS, reset: RESET, frequency: i64, delay: DELAY) -> Self {
        Self::do_abp(
            Self::setup_lora(spi, cs, reset, frequency, delay),
            nonce::Nonce::new(),
        )
    }

    pub async fn send_message(&mut self, message: &[u8]) -> Result<(), ()> {
        let mut phy = lorawan::creator::DataPayloadCreator::new();
        phy.set_confirmed(true)
            .set_uplink(true)
            .set_f_port(42)
            .set_dev_addr(self.dev_addr)
            .set_fctrl(&lorawan::parser::FCtrl::new(0x80, true)) // ADR: true, all others: false
            .set_fcnt(17);
        let payload = phy
            .build(message, &[], &self.nwk_skey, &self.app_skey)
            .unwrap();

        self.send_raw_message(payload).await
    }
}
