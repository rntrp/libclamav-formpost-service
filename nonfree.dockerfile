FROM rust:1-slim-bookworm AS build
WORKDIR /app
COPY Cargo.lock Cargo.toml build.rs ./
RUN echo "deb http://deb.debian.org/debian bookworm-backports main" > /etc/apt/sources.list.d/backports.list \
    && apt update \
    && apt install -y pkg-config libclang-dev libclamav-dev upx-ucl \
    && mkdir src \
    && echo "fn main() {}" > src/main.rs \
    && cargo build --profile release-opt \
    && rm src/main.rs
COPY src src
RUN touch -a -m src/main.rs \
    && cargo build --profile release-opt \
    && upx --best --lzma target/release-opt/libclamav-formpost-service

FROM debian:bookworm-slim
RUN sed -i -e "s/ main/ main contrib non-free/g" /etc/apt/sources.list.d/debian.sources \
    && apt update -qq \
    && apt install -y --no-install-recommends libclamav11 libclamunrar11 clamav-freshclam ca-certificates \
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
