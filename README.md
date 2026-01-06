[![Release](https://img.shields.io/github/v/release/rntrp/libclamav-formpost-service?include_prereleases)](https://github.com/rntrp/libclamav-formpost-service/releases)
[![Docker Image](https://img.shields.io/docker/image-size/rntrp/libclamav-formpost-service/latest?logo=docker)](https://hub.docker.com/r/rntrp/libclamav-formpost-service)

# libclamav-formpost-service 🪸
Antivirus formpost microservice based on [async libclamav bindings](https://github.com/Cisco-Talos/clamav-async-rs), written in Rust using [Axum](https://github.com/tokio-rs/axum).

## Build
The program requires the `libclamav` dynamic library at runtime and the corresponding header files for building:
* At least ClamAV 1.4.x libs and header files, including `/usr/lib**/libclamav.so.12`, `/usr/include/clamav.h` etc.
* Those files must be available via `pkg-config`, i.e. `/usr/lib**/pkgconfig/libclamav.pc` must be present
* Rust must be [installed](https://www.rust-lang.org/tools/install)

The most convenient option is to install `libclamav` development files from official package sources:
* __Debian Trixie__: [libclamav-dev](https://packages.debian.org/trixie/libclamav-dev)
* __Ubuntu Noble__: [libclamav-dev](https://launchpad.net/ubuntu/noble/+package/libclamav-dev)
* __Arch Linux__: [clamav](https://archlinux.org/packages/extra/x86_64/clamav/)
* __Fedora 42+__ or __EPEL8+__ [clamav-devel](https://packages.fedoraproject.org/pkgs/clamav/clamav-devel/)

Then just do `cargo build` or straightaway `cargo run` on the repo root so that cargo automatically downloads all the dependencies and builds the binary.

## Launch

### From Source Files
Just use `cargo` from the repo root. Make sure all the prerequisites are met.
```sh
cargo run
```

### As Docker Container
```sh
docker run --rm -p 8000:8000 rntrp/libclamav-formpost-service
```
Or pull from the GitHub Registry in place of Docker Hub:
```sh
docker run --rm -p 8000:8000 ghcr.io/rntrp/libclamav-formpost-service
```

## Usage
* `/` leads to a simple HTML page with a form upload
* `/health` is a simple health check endpoint
* `/metrics` provides metrics in Prometheus format
* `/shutdown` initiates graceful shutdown on a POST request (disabled by default)
* `/upload` will accept files via POST `multipart/form-data` request. Returns a JSON after upload and scan:
```jsonc
{
  "avVersion": "1.5.1",
  "dbVersion": 27871,
  "dbSignatureCount": 3627117,
  "dbDate": "2026-01-05T07:25:47.000Z",
  "results": [
    {
      "name": "eicar_com.zip",
      "size": 184,
      "crc32": "31db20d1",
      "md5": "6ce6f415d8475545be5ba114f208b0ff",
      "sha256": "2546dcffc5ad854d4ddc64fbf056871cd5a00f2471cb7a5bfd4ac23b6e9eedad",
      "contentType": "application/zip",
      "dateScanned": "2026-01-06T20:27:30.991Z",
      "result": "VIRUS", // or CLEAN or WHITELISTED
      "signature": "Eicar-Test-Signature" // null if CLEAN
    }
  ]
}
```
