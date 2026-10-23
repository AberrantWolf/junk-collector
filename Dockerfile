FROM rust:1.99.1-slim-bookworm AS builder
RUN apt-get update && apt-get install -y --no-install-recommends cmake pkg-config libssl-dev git ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY web/dist ./web/dist
RUN cargo build --locked --release -p collector-server -p collector-media

# Optional lean target. The default release image below contains both executables
# so the same immutable release can run the core and isolated media containers.
FROM debian:bookworm-slim AS media
RUN useradd --uid 10001 --create-home collector
COPY --from=builder /build/target/release/collector-media /usr/local/bin/collector-media
USER collector
ENTRYPOINT ["collector-media"]

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates libssl3 flac opus-tools && rm -rf /var/lib/apt/lists/* && useradd --uid 10001 --create-home collector
COPY --from=builder /build/target/release/collector-server /usr/local/bin/junk-collector
COPY --from=builder /build/target/release/collector-media /usr/local/bin/collector-media
COPY --chmod=755 deploy/container/initialize.sh /usr/local/bin/collector-initialize
COPY --chmod=755 deploy/container/initialize-media.sh /usr/local/bin/collector-initialize-media
COPY --chmod=755 deploy/container/install-proxy-config.sh /usr/local/bin/collector-install-proxy-config
LABEL org.opencontainers.image.source="https://github.com/AberrantWolf/junk-collector"
USER collector
WORKDIR /home/collector
EXPOSE 9843 9844 9845/tcp 9845/udp
STOPSIGNAL SIGTERM
ENTRYPOINT ["junk-collector"]
CMD ["--config", "/config/collector.toml", "serve"]
