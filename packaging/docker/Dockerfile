# Apeireth 2.0 Dockerfile (Multi-stage build)
# Image: apeireth/apeireth:2.0.0-rc.1
# Canonical gateway serve on port 8080

FROM rust:1.97.1-bookworm AS builder

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        ca-certificates \
        libdbus-1-dev \
        pkg-config \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates

RUN cargo build --release --workspace --bin apeireth --locked \
    && strip target/release/apeireth

FROM debian:bookworm-slim AS runtime

LABEL org.opencontainers.image.title="Apeireth OS" \
      org.opencontainers.image.description="Apeireth OS 2.0 - AGI Operating System & Cognitive Microkernel" \
      org.opencontainers.image.version="2.0.0-rc.1" \
      org.opencontainers.image.url="https://github.com/apeireth/apeireth-rust" \
      org.opencontainers.image.licenses="Apache-2.0"

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        ca-certificates \
        curl \
        libdbus-1-3 \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --create-home --uid 10001 --shell /usr/sbin/nologin apeireth

COPY --from=builder /build/target/release/apeireth /usr/local/bin/apeireth

ENV APEIRETH_HOME=/var/lib/apeireth \
    APEIRETH_API_KEY="" \
    APEIRETH_BASE_URL=https://api.minimaxi.com/v1 \
    APEIRETH_MODEL=MiniMax-M3 \
    RUST_LOG=info

RUN mkdir -p /var/lib/apeireth /var/log/apeireth /etc/apeireth \
    && chown -R apeireth:apeireth /var/lib/apeireth /var/log/apeireth /etc/apeireth

USER apeireth
WORKDIR /var/lib/apeireth
EXPOSE 8080

HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD curl --fail --silent http://127.0.0.1:8080/health || exit 1

ENTRYPOINT ["/usr/local/bin/apeireth"]
CMD ["gateway", "serve", "--port", "8080"]
