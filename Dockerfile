FROM rust:1.95-slim-bookworm AS builder
RUN apt-get update && apt-get install -y --no-install-recommends cmake pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*
WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY web/dist ./web/dist
RUN cargo build --locked --release -p collector-server

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates libssl3 && rm -rf /var/lib/apt/lists/* && useradd --uid 10001 --create-home collector
COPY --from=builder /build/target/release/collector-server /usr/local/bin/junk-collector
USER collector
WORKDIR /home/collector
EXPOSE 9843
ENTRYPOINT ["junk-collector"]
CMD ["--config", "/config/collector.toml", "serve"]
