FROM rust:1.77 as builder

WORKDIR /usr/src/invoke-rs

COPY Cargo.toml Cargo.lock ./
COPY src ./src

RUN cargo build --release

FROM debian:bookworm-slim

WORKDIR /usr/src/invoke-rs

# curl is used for docker-compose health checks
RUN apt-get update && \
    apt-get install curl ca-certificates -y --no-install-recommends && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

COPY --from=builder /usr/src/invoke-rs/target/release/invoke-rs /usr/bin/invoke-rs

CMD [ "invoke-rs" ]