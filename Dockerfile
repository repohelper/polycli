# CodexCTL - Docker Image
# Multi-stage build for minimal size

# Stage 1: Build
FROM rust:1.94-slim-bookworm AS builder

WORKDIR /usr/src/codexctl

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
LABEL description="CodexCTL - Codex CLI Profile Manager"
LABEL version="0.1.0"

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/* \
    && apt-get clean

# Create non-root user
RUN useradd -m -s /bin/bash codexctl

# Copy binary from builder
COPY --from=builder /usr/src/codexctl/target/release/codexctl /usr/local/bin/codexctl
COPY --from=builder /usr/src/codexctl/target/release/cdx /usr/local/bin/cdx

# Set permissions
RUN chmod +x /usr/local/bin/codexctl /usr/local/bin/cdx

# Create profiles directory
RUN mkdir -p /home/codexctl/.local/share/codexctl && \
    chown -R codexctl:codexctl /home/codexctl

# Switch to non-root user
USER codexctl

# Set environment
ENV CODEXCTL_DIR=/home/codexctl/.local/share/codexctl
ENV RUST_LOG=info

# Volume for persistent profiles
VOLUME ["/home/codexctl/.local/share/codexctl"]

# Default command
ENTRYPOINT ["codexctl"]
CMD ["--help"]
