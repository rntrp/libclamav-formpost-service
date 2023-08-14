[![Release](https://img.shields.io/github/v/release/rntrp/libclamav-formpost-service)](https://github.com/rntrp/libclamav-formpost-service/releases)
[![Docker Image](https://img.shields.io/docker/image-size/rntrp/libclamav-formpost-service/latest?logo=docker)](https://hub.docker.com/r/rntrp/libclamav-formpost-service)

# libclamav-formpost-service
Antivirus formpost microservice based on [libclamav](https://github.com/Cisco-Talos/clamav), written in Rust using [Axum](https://github.com/tokio-rs/axum).

Parts of the code were taken from [clamav-rust](https://github.com/zaddach/clamav-rs) by Jonas Zaddach. Kudos to Jonas for FFI bindings and Settings implementation based on [bitflags](https://github.com/bitflags/bitflags).

Build is currenty targeting only x86-64 Linux systems.
