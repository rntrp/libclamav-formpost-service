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
RUN apt update \
    && apt install libclamav11 clamav-freshclam -y
COPY --from=build /app/target/release/libclamav-formpost-service ./
EXPOSE 8000
ENV RUST_LOG=DEBUG
CMD freshclam -V; freshclam; ./libclamav-formpost-service
