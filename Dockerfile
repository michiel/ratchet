# Multi-stage build for optimal image size
FROM rust:1.75-bookworm as builder

# Build arguments
ARG VERSION=dev
ARG BUILD_DATE
ARG VCS_REF

# Install system dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Create app user
RUN useradd -m -u 1000 ratchet

# Set working directory
WORKDIR /usr/src/ratchet

# Copy Cargo files first for better layer caching
COPY Cargo.toml Cargo.lock ./
COPY ratchet-*/Cargo.toml ./
RUN mkdir -p ratchet-interfaces/src ratchet-api-types/src ratchet-web/src \
    ratchet-rest-api/src ratchet-graphql-api/src ratchet-server/src \
    ratchet-core/src ratchet-runtime/src ratchet-storage/src \
    ratchet-ipc/src ratchet-resilience/src ratchet-caching/src \
    ratchet-plugins/src ratchet-execution/src ratchet-http/src \
    ratchet-js/src ratchet-logging/src ratchet-output/src \
    ratchet-mcp/src ratchet-cli-tools/src ratchet-cli/src \
    ratchet-config/src ratchet-plugin/src ratchet-registry/src \
    tests/src && \
    echo "fn main() {}" > ratchet-cli/src/main.rs && \
    find . -name "Cargo.toml" -path "*/ratchet-*" -exec dirname {} \; | \
    xargs -I {} sh -c 'echo "fn main() {}" > {}/src/lib.rs || true'

# Build dependencies (this layer will be cached unless Cargo.toml changes)
RUN cargo build --release --bin ratchet && \
    rm -rf target/release/deps/ratchet* && \
    rm -rf ratchet-*/src

# Copy source code
COPY . .

# Build the application with optimized profile
RUN cargo build --profile dist --bin ratchet

# Runtime image
FROM debian:bookworm-slim

# Build arguments for labels
ARG VERSION=dev
ARG BUILD_DATE
ARG VCS_REF

# Add labels
LABEL org.opencontainers.image.title="Ratchet"
LABEL org.opencontainers.image.description="Task automation and execution platform with GraphQL, REST, and MCP APIs"
LABEL org.opencontainers.image.version="$VERSION"
LABEL org.opencontainers.image.created="$BUILD_DATE"
LABEL org.opencontainers.image.revision="$VCS_REF"
LABEL org.opencontainers.image.vendor="Ratchet Project"
LABEL org.opencontainers.image.licenses="MIT OR Apache-2.0"
LABEL org.opencontainers.image.source="https://github.com/michiel/ratchet-workspace"
LABEL org.opencontainers.image.documentation="https://github.com/michiel/ratchet-workspace"

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/* \
    && apt-get clean

# Create app user
RUN useradd -m -u 1000 ratchet

# Create application directories
RUN mkdir -p /app/data /app/logs /app/config && \
    chown -R ratchet:ratchet /app

# Copy binary from builder stage
COPY --from=builder /usr/src/ratchet/target/dist/ratchet /usr/local/bin/ratchet

# Set permissions
RUN chmod +x /usr/local/bin/ratchet

# Switch to non-root user
USER ratchet

# Set working directory
WORKDIR /app

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD ratchet --version || exit 1

# Default configuration
ENV RUST_LOG=info
ENV RATCHET_DATA_DIR=/app/data
ENV RATCHET_LOG_DIR=/app/logs
ENV RATCHET_CONFIG_DIR=/app/config

# Expose default ports
# REST API
EXPOSE 8080
# GraphQL API  
EXPOSE 8081
# MCP Server (if using HTTP transport)
EXPOSE 8082

# Default volumes
VOLUME ["/app/data", "/app/logs", "/app/config"]

# Entry point
ENTRYPOINT ["ratchet"]
CMD ["--help"]