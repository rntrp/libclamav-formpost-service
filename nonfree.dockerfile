FROM rust:1-slim-bookworm AS build
WORKDIR /app
COPY Cargo.lock Cargo.toml build.rs ./
RUN apt update \
    && apt install pkg-config libclang-dev libclamav-dev -y \
    && mkdir src \
    && echo "fn main() {}" > src/main.rs \
    && cargo build --release \
    && rm src/main.rs
COPY src src
RUN touch -a -m src/main.rs \
    && cargo build --release

FROM debian:bookworm-slim
RUN sed -i -e "s/ main/ main contrib non-free/g" /etc/apt/sources.list.d/debian.sources \
    && apt update \
    && apt install libclamav11 libclamunrar11 clamav-freshclam -y \
    && freshclam
COPY --from=build /app/target/release/rust-axum-clamav ./
EXPOSE 8000
ENV RUST_LOG=DEBUG
CMD freshclam -V; freshclam; ./rust-axum-clamav
