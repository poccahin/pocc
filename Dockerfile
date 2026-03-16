# POCC Runtime — Rust Workspace Image
FROM rust:1.81-slim AS builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy workspace manifests first for layer caching
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/

# Build all workspace crates in release mode
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy compiled binaries from builder
COPY --from=builder /app/target/release/ ./bin/

LABEL org.opencontainers.image.title="POCC Runtime" \
      org.opencontainers.image.description="Life++ Planetary Core — Rust workspace runtime" \
      org.opencontainers.image.source="https://github.com/poccahin/pocc"
