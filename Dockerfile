# SPDX-FileCopyrightText: 2025 Semiotic Labs
#
# SPDX-License-Identifier: Apache-2.0

# Multi-stage Dockerfile for NFT API production deployment

# Build stage
FROM rust:1.89-slim as builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/* \
    && apt-get clean

# Set working directory
WORKDIR /app

# Copy workspace configuration files for dependency resolution
COPY Cargo.toml Cargo.lock ./
COPY clippy.toml rustfmt.toml deny.toml ./

# Create placeholder source files to enable dependency pre-building
RUN mkdir -p crates/api/src crates/api-client/src crates/external-apis/src crates/shared-types/src crates/spam-predictor/src

# Copy all Cargo.toml files for each crate
COPY crates/api/Cargo.toml crates/api/
COPY crates/api-client/Cargo.toml crates/api-client/
COPY crates/external-apis/Cargo.toml crates/external-apis/
COPY crates/shared-types/Cargo.toml crates/shared-types/
COPY crates/spam-predictor/Cargo.toml crates/spam-predictor/

# Create placeholder main.rs files to satisfy cargo build
RUN echo 'fn main() {}' > crates/api/src/main.rs && \
    echo 'fn main() {}' > crates/api/src/lib.rs && \
    echo '' > crates/api-client/src/lib.rs && \
    echo '' > crates/external-apis/src/lib.rs && \
    echo '' > crates/shared-types/src/lib.rs && \
    echo '' > crates/spam-predictor/src/lib.rs

# Build dependencies only (this layer will be cached)
RUN cargo build --release --bin api

# Remove placeholder files and copy actual source code
RUN rm -rf crates/*/src/
COPY crates/ crates/

# Build the actual application with all optimizations
# Touch source files to ensure they're newer than the dependency build
RUN find crates -name "*.rs" -exec touch {} + && \
    cargo build --release --bin api

# Runtime stage
FROM debian:bookworm-slim as runtime

# Install runtime dependencies including curl for healthcheck
RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/* \
    && apt-get clean

# Create a non-root user with specific UID/GID for security
RUN groupadd --gid 1000 appuser && \
    useradd --uid 1000 --gid 1000 --create-home --shell /bin/bash appuser

# Create app directory with proper permissions
RUN mkdir -p /app && chown appuser:appuser /app

# Set working directory
WORKDIR /app

# Copy the binary from builder stage with proper ownership
COPY --from=builder --chown=appuser:appuser /app/target/release/api /app/api

# Copy assets directory
COPY --chown=appuser:appuser assets/ /app/assets/

# Ensure binary is executable
RUN chmod +x /app/api

# Switch to non-root user
USER appuser

# Expose the default port
EXPOSE 3000

# Set environment variables for production
ENV RUST_LOG=info
ENV ENVIRONMENT=production
ENV RUST_BACKTRACE=0
ENV CONFIG_FILE=/app/config.production.json
ENV SERVER__HOST=0.0.0.0
# Security: Disable core dumps and limit resources
ENV RLIMIT_CORE=0
# Performance: Set optimal thread count (will be overridden by actual CPU count)
ENV TOKIO_WORKER_THREADS=4

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:3000/health || exit 1

# Run the application
CMD ["./api"]
