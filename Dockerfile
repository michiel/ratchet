# Multi-stage build for optimal image size
FROM rust:1.86-bookworm as builder

# Build arguments
ARG VERSION=dev
ARG BUILD_DATE
ARG VCS_REF

# Cargo build optimizations
ARG CARGO_REGISTRIES_CRATES_IO_PROTOCOL=sparse
ARG CARGO_INCREMENTAL=0
ARG RUST_BACKTRACE=1
ENV CARGO_REGISTRIES_CRATES_IO_PROTOCOL=$CARGO_REGISTRIES_CRATES_IO_PROTOCOL
ENV CARGO_INCREMENTAL=$CARGO_INCREMENTAL
ENV RUST_BACKTRACE=$RUST_BACKTRACE

# Install system dependencies (cached layer - rarely changes)
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Create app user
RUN useradd -m -u 1000 ratchet

# Set working directory
WORKDIR /usr/src/ratchet

# Copy only dependency files first (better caching)
COPY Cargo.toml Cargo.lock ./
COPY ratchet-js/Cargo.toml ./ratchet-js/
COPY ratchet-output/Cargo.toml ./ratchet-output/
COPY ratchet-cli-tools/Cargo.toml ./ratchet-cli-tools/
COPY ratchet-graphql-api/Cargo.toml ./ratchet-graphql-api/
COPY ratchet-api-types/Cargo.toml ./ratchet-api-types/
COPY ratchet-plugin/Cargo.toml ./ratchet-plugin/
COPY ratchet-logging/Cargo.toml ./ratchet-logging/
COPY ratchet-interfaces/Cargo.toml ./ratchet-interfaces/
COPY ratchet-web/Cargo.toml ./ratchet-web/
COPY ratchet-ipc/Cargo.toml ./ratchet-ipc/
COPY ratchet-storage/Cargo.toml ./ratchet-storage/
COPY ratchet-caching/Cargo.toml ./ratchet-caching/
COPY ratchet-config/Cargo.toml ./ratchet-config/
COPY ratchet-plugins/Cargo.toml ./ratchet-plugins/
COPY ratchet-resilience/Cargo.toml ./ratchet-resilience/
COPY ratchet-http/Cargo.toml ./ratchet-http/
COPY ratchet-registry/Cargo.toml ./ratchet-registry/
COPY ratchet-core/Cargo.toml ./ratchet-core/
COPY ratchet-rest-api/Cargo.toml ./ratchet-rest-api/
COPY ratchet-execution/Cargo.toml ./ratchet-execution/
COPY ratchet-cli/Cargo.toml ./ratchet-cli/
COPY ratchet-server/Cargo.toml ./ratchet-server/
COPY ratchet-runtime/Cargo.toml ./ratchet-runtime/
COPY ratchet-mcp/Cargo.toml ./ratchet-mcp/

# Create dummy source files for dependency compilation
RUN find . -name "Cargo.toml" -not -path "./target/*" -exec dirname {} \; | \
    xargs -I {} sh -c 'mkdir -p {}/src && \
    if [ -f {}/Cargo.toml ] && grep -q "\[\[bin\]\]" {}/Cargo.toml; then \
        echo "fn main() {}" > {}/src/main.rs; \
    else \
        echo "pub fn main() {}" > {}/src/lib.rs; \
    fi'

# Build dependencies only (cached layer)
RUN cargo build --profile dist --bin ratchet --locked
RUN rm -rf src/ ratchet-*/src/

# Copy actual source code
COPY . .

# Touch source files to ensure rebuild
RUN find . -name "*.rs" -exec touch {} \;

# Build the application with optimized profile
RUN cargo build --profile dist --bin ratchet --locked

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
LABEL org.opencontainers.image.source="https://github.com/ratchet-runner/ratchet-workspace"
LABEL org.opencontainers.image.documentation="https://github.com/ratchet-runner/ratchet-workspace"

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