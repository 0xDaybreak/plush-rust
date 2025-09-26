FROM rust:1.89 as builder

WORKDIR /usr/src/plush_rust
COPY . .

RUN cargo install --path .

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y libssl-dev ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /usr/local/cargo/bin/plush_rust /usr/local/bin/plush_rust

EXPOSE 8080
CMD ["plush_rust"]
