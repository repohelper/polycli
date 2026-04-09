# Codexo - Docker Image
# Multi-stage build for minimal size

# Stage 1: Build
FROM rust:1.94-slim-bookworm AS builder

WORKDIR /usr/src/codexo

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Copy source code
COPY src ./src

# Build release binary
RUN cargo build --release

# Stage 2: Runtime
FROM debian:bookworm-slim

LABEL maintainer="Bhanu Korthiwada"
LABEL description="Codexo - The Ultimate Codex Profile Manager"
LABEL version="0.1.0"

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/* \
    && apt-get clean

# Create non-root user
RUN useradd -m -s /bin/bash codex

# Copy binary from builder
COPY --from=builder /usr/src/codexo/target/release/codexo /usr/local/bin/codexo
COPY --from=builder /usr/src/codexo/target/release/cx /usr/local/bin/cx

# Set permissions
RUN chmod +x /usr/local/bin/codexo /usr/local/bin/cx

# Create profiles directory
RUN mkdir -p /home/codex/.local/share/codexo && \
    chown -R codex:codex /home/codex

# Switch to non-root user
USER codex

# Set environment
ENV CODEXO_DIR=/home/codex/.local/share/codexo
ENV RUST_LOG=info

# Volume for persistent profiles
VOLUME ["/home/codex/.local/share/codexo"]

# Default command
ENTRYPOINT ["codexo"]
CMD ["--help"]
