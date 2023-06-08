## Remote MPPT measurement and logging system for solar modules

This repository contains the code for a remote measurement system for PV modules. It also contains the KiCad files which were used to create the PCB.

### Configuration

Before compiling, edit the encryption key in `src/encryption/mod.rs`. If you don't do this you will get a compile error. Edit this once and keep it the same for all senders and receivers.

The code takes a number of configuration parameters. For the sender, these are:

- USE_DISPLAY: Set this to false to disable all communications with the screen (in case the device does not have a screen for example).
- NONCE_MIN/NONCE_MAX: A range of nonces to use for this device. Both must be between 0 and 65535. The range should be unique for each sender such that there are no overlaps, and the range should be at least 200 long. For example, for one device NONCE_MIN=1000, NONCE_MAX=1199, and for another device NONCE_MIN=1200, NONCE_MAX=1399.
- DEVICE_ID: A unique id in the range 0-127. The InfluxDB "host" field will be set to "ttgo<DEVICE_ID>", for example "ttgo25" for DEVICE_ID=25.

For the receiver, the only configuration parameter is USE_DISPLAY.

The configuration parameters are passed as environment variables to the `cargo build` command, or as build arguments to Docker.

## Compiling

In order to not have to install a bunch of stuff (ESP-IDF development framework and forked version of rust with ESP32 support), the program can be compiled in Docker.

Before continuing, make sure to install Docker engine, and that the `docker` command (or `sudo docker`) is available.

The first time you run any of the following commands it takes a long time (15 minutes), but it shouldn't take too long the next time.

### Sender

To compile the sender, run the following command in this repository:

`docker build . -t measurement-sender --build-arg FEATURES=sender --build-arg USE_DISPLAY=true --build-arg NONCE_MIN=1000 --build-arg NONCE_MAX=1199 --build-arg DEVICE_ID=75`

(Edit the values USE_DISPLAY, NONCE_MIN, NONCE_MAX and DEVICE_ID as necessary before each compile).

You now have a Docker image named `measurement-sender` on your computer. It contains the built code.

Now run one of the following:

- Linux: `docker run -v .:/dir -it measurement-sender cp /usr/src/app/target/xtensa-esp32-espidf/release/pv-measurement-system /dir/sender.elf`
- Windows : `docker run -v ${PWD}:/dir -it measurement-sender cp /usr/src/app/target/xtensa-esp32-espidf/release/pv-measurement-system /dir/sender.elf`

This will copy the built code to the file `sender.elf` in your current directory, which you can load onto the ESP32. One way of doing this is:

- Install Rust (https://rustup.rs)
- Run `cargo install espflash`
- Run `espflash --monitor ./sender.elf`.

### Receiver

To compile the sender, run the following command in this repository:

`docker build . -t measurement-receiver --build-arg FEATURES=receiver --build-arg USE_DISPLAY=true`

(Edit the value USE_DISPLAY as necessary before each compile).

You now have a Docker image named `measurement-receiver` on your computer. It contains the built code.

Now run one of the following:

- Linux: `docker run -v .:/dir -it measurement-receiver cp /usr/src/app/target/xtensa-esp32-espidf/release/pv-measurement-system /dir/receiver.elf`
- Windows : `docker run -v ${PWD}:/dir -it measurement-receiver cp /usr/src/app/target/xtensa-esp32-espidf/release/pv-measurement-system /dir/receiver.elf`

This will copy the built code to the file `receiver.elf` in your current directory, which you can load onto the ESP32. One way of doing this is:

- Install Rust (https://rustup.rs)
- Run `cargo install espflash`
- Run `espflash --monitor ./receiver.elf`.

### Without Docker

If you do not want to compile in docker, follow the instructions at `https://esp-rs.github.io/book/installation/index.html` to install the required tools. Here's a quick summary:

- Install rust (https://rustup.rs/)
- Install XTensa target: `cargo install espup && espup install`
- Install espflash: `cargo install cargo-espflash`

On windows, we also had to install ESP-IDF under "manual installation" at (https://docs.espressif.com/projects/esp-idf/en/latest/esp32/get-started/).

Also, make sure to "source" you current shell:

- Linux: `. ~/export-esp.sh`
- Windows: We had to run inside the ESP_IDF development environment.

Then, to compile and load the code onto the device simply run:

Linux:

- Sender: `USE_DISPLAY=true NONCE_MIN=1000 NONCE_MAX=1199 DEVICE_ID=75 cargo run --features sender`
- Receiver: `USE_DISPLAY=true cargo run --features receiver`

Windows:

- Sender: `$env:USE_DISPLAY = "true"; $env:NONCE_MIN = 1000; $env:NONCE_MAX = 1199; $env:DEVICE_ID = 75; cargo run --features sender`
- Receiver: `$env:USE_DISPLAY = "true" cargo run --features receiver`