FROM rust:1.98.0-bookworm AS build

WORKDIR /app

COPY . .

RUN cargo build --release

FROM debian:13-slim

RUN apt-get update

RUN apt-get install -y --no-install-recommends ca-certificates curl

RUN apt-get clean

RUN rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=build /app/target/release/rust-api .

HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD curl --fail --silent http://127.0.0.1:8686/health/api || exit 1

CMD ["/app/rust-api"]
