# Canonical Apeireth workspace image.
# The image contains the root-workspace CLI and starts the canonical gateway.

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

RUN mkdir -p /var/lib/apeireth \
    && chown -R apeireth:apeireth /var/lib/apeireth

USER apeireth
WORKDIR /var/lib/apeireth
EXPOSE 8080

HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD curl --fail --silent http://127.0.0.1:8080/health || exit 1

ENTRYPOINT ["/usr/local/bin/apeireth"]
CMD ["gateway", "serve", "--port", "8080"]
