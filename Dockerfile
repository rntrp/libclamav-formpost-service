FROM rust:1-slim-trixie AS build
WORKDIR /app
COPY Cargo.lock Cargo.toml build.rs ./
RUN apt update \
    && apt install -y pkg-config libclang-dev libclamav-dev upx-ucl \
    && mkdir src \
    && echo "fn main() {}" > src/main.rs \
    && cargo build --profile release-opt \
    && rm src/main.rs
COPY src src
RUN touch -a -m src/main.rs \
    && cargo build --profile release-opt \
    && upx --best --lzma target/release-opt/libclamav-formpost-service

FROM debian:trixie-slim
RUN apt update -qq \
    && apt install -y --no-install-recommends libclamav12 clamav-freshclam ca-certificates \
    && apt autoremove --purge \
    && apt clean \
    && rm -rf /var/lib/apt/lists /var/cache/apt/archives \
    && mkdir -p /var/lib/clamav /var/log/clamav \
    && chown -R 1001:1001 /var/lib/clamav /var/log/clamav
COPY --from=build /app/target/release-opt/libclamav-formpost-service ./
EXPOSE 8000
ENV RUST_LOG=DEBUG
USER 1001
CMD freshclam -V; freshclam; ./libclamav-formpost-service
