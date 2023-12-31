[![Release](https://img.shields.io/github/v/release/rntrp/libclamav-formpost-service?include_prereleases)](https://github.com/rntrp/libclamav-formpost-service/releases)
[![Docker Image](https://img.shields.io/docker/image-size/rntrp/libclamav-formpost-service/latest?logo=docker)](https://hub.docker.com/r/rntrp/libclamav-formpost-service)

# libclamav-formpost-service
Antivirus formpost microservice based on [libclamav](https://github.com/Cisco-Talos/clamav), written in Rust using [Axum](https://github.com/tokio-rs/axum).

Parts of the code were taken from [clamav-rust](https://github.com/zaddach/clamav-rs) by Jonas Zaddach. Kudos to Jonas for FFI bindings and Settings implementation based on [bitflags](https://github.com/bitflags/bitflags).

Build is currenty targeting only x86-64 Linux systems.

## Build
The program is bound to ClamAV 1.0.x LTS releases. `libclamav` changes its API with later stable versions, so the latest LTS is used instead. Prerequisites are:
* ClamAV 1.0.x header files and libs
* Those files must be available via `pkg-config`
* Rust must be [installed](https://www.rust-lang.org/tools/install)

Currently the most convenient option is to build it Debian Bookworm with [libclamav-dev](https://packages.debian.org/bookworm/libclamav-dev).

Then just do `cargo build` or even `cargo run` on the repo root so that cargo automatically downloads all the dependencies and builds the binary.

## Launch
The program is intended mainly for use within a Docker container:
```sh
docker run --rm -p 8000:8000 rntrp/libclamav-formpost-service
```
Or pull from the GitHub Registry in place of Docker Hub:
```sh
docker run --rm -p 8000:8000 ghcr.io/rntrp/libclamav-formpost-service
```

## Usage
* `/` leads to a simple HTML page with a form upload
* `/upload` will accept files via POST `multipart/form-data` request. Returns a JSON after upload and scan:
```json
{
  "avVersion": "1.0.3",
  "dbVersion": 27077,
  "dbSignatureCount": 8677120,
  "dbDate": "2023-10-30T07:39:55.000Z",
  "results": [
    {
      "name": "eicar_com.zip",
      "size": 184,
      "crc32": "31db20d1",
      "md5": "6ce6f415d8475545be5ba114f208b0ff",
      "sha256": "2546dcffc5ad854d4ddc64fbf056871cd5a00f2471cb7a5bfd4ac23b6e9eedad",
      "contentType": "application/zip",
      "dateScanned": "2023-10-30T18:57:36.763Z",
      "result": "VIRUS", // or CLEAN or WHITELISTED
      "signature": "Win.Test.EICAR_HDB-1" // null if CLEAN
    }
  ]
}
```
* `/health` is a simple health check endpoint
* `/metrics` provides metrics in Prometheus format
* `/shutdown` initiates graceful shutdown on a POST request
