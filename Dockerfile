FROM rust:1.70

WORKDIR /usr/src/app

RUN cargo install espup && espup install && cargo install ldproxy

RUN apt update && apt install python3 python3-pip -y

COPY .cargo .cargo
COPY build.rs build.rs
COPY Cargo.lock Cargo.lock
COPY Cargo.toml Cargo.toml
COPY rust-toolchain.toml rust-toolchain.toml
COPY sdkconfig.defaults sdkconfig.defaults

RUN . ~/export-esp.sh && mkdir src && echo "use esp_idf_sys as _; fn main() {}" > src/main.rs && cargo build --release && rm -rf src

COPY src src

ARG FEATURES
ARG NONCE_MIN
ARG NONCE_MAX
ARG USE_DISPLAY
ARG DEVICE_ID

RUN . ~/export-esp.sh && cargo build --release --features $FEATURES