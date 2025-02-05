FROM rust:1.84 as builder

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY src ./src

RUN cargo build --release
FROM debian:bookworm-slim

WORKDIR /app
COPY --from=builder /app/target/release/paranoia_sysinfo .
