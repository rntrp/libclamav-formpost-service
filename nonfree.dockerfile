FROM rust:1-slim-trixie AS build
WORKDIR /app
COPY Cargo.lock Cargo.toml build.rs ./
RUN sed -i -e "s/ main/ main contrib non-free/g" /etc/apt/sources.list.d/debian.sources \
    && apt update -qq \
    && apt install -y pkg-config libclang-dev libclamav-dev libclamunrar clamav-freshclam upx-ucl \
    && mkdir /libs \
    && cp -v -s /usr/lib/$(gcc -dumpmachine)/*.so.* /libs \
    && mkdir src \
    && echo "fn main() {}" > src/main.rs \
    && cargo build --profile release-opt \
    && rm src/main.rs
COPY src src
RUN touch -a -m src/main.rs \
    && cargo build --profile release-opt \
    && upx --best --lzma target/release-opt/libclamav-formpost-service

FROM gcr.io/distroless/base-nossl-debian13:nonroot
COPY --from=build \
    /libs/libclamav.so.12 \
    /libs/libclamunrar.so.12 \
    /libs/libclamunrar_iface.so.12 \
    /libs/libmspack.so.0 \
    /libs/libcrypto.so.3 \
    /libs/libz.so.1 \
    /libs/libbz2.so.1.0 \
    /libs/libpcre2-8.so.0 \
    /libs/libxml2.so.2 \
    /libs/libjson-c.so.5 \
    /libs/libm.so.6 \
    /libs/libgcc_s.so.1 \
    /libs/libc.so.6 \
    /libs/libzstd.so.1 \
    /libs/liblzma.so.5 \
    /usr/lib/
COPY --from=build /app/target/release-opt/libclamav-formpost-service ./
EXPOSE 8000
ENV RUST_LOG=DEBUG
ENTRYPOINT ["./libclamav-formpost-service"]
