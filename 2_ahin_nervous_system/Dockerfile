# L1 AHIN CR+ Router — Rust P2P Gateway & WebSocket Broadcast Tower
FROM rust:1.81-slim AS builder

WORKDIR /app/cr_plus_tensor_routing

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    protobuf-compiler \
    && rm -rf /var/lib/apt/lists/*

# Copy workspace manifests for layer caching
COPY cr_plus_tensor_routing/Cargo.toml ./
COPY cr_plus_tensor_routing/Cargo.lock* ./

# Pre-fetch dependencies with a placeholder binary to populate the cargo cache.
# The build is allowed to fail here since the real source is not yet copied;
# errors are suppressed intentionally to avoid false failures in this cache-warm step.
RUN mkdir -p src && echo "fn main() {}" > src/main.rs && \
    cargo build --release 2>/dev/null || true && \
    rm -rf src

# Copy actual source and build in release mode
COPY cr_plus_tensor_routing/src/ ./src/
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/cr_plus_tensor_routing/target/release/cr_plus_tensor_routing ./

EXPOSE 8000 9000

ENV RUST_LOG=info

CMD ["./cr_plus_tensor_routing"]

LABEL org.opencontainers.image.title="AHIN CR+ Router" \
      org.opencontainers.image.description="Life++ L1 CR+ Tensor Routing Gateway" \
      org.opencontainers.image.source="https://github.com/poccahin/pocc"
