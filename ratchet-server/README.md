# Ratchet Server

A unified server that combines REST and GraphQL APIs with all necessary services, demonstrating the new modular Ratchet architecture.

## Overview

The Ratchet Server showcases the successful migration from a monolithic `ratchet-lib` to a modular architecture with:

- **REST API** via `ratchet-rest-api` 
- **GraphQL API** via `ratchet-graphql-api` (temporarily disabled during migration)
- **Shared middleware** via `ratchet-web`
- **Unified types** via `ratchet-api-types`
- **Dependency injection** via `ratchet-interfaces`
- **Graceful shutdown** and signal handling
- **Comprehensive configuration** with CLI and file support

## Features

### Server Configuration
- HTTP server binding and middleware controls
- REST API configuration with health checks
- GraphQL API configuration with playground support  
- Logging configuration with file and structured output
- Database connection configuration
- Task registry configuration with filesystem and HTTP sources

### API Endpoints
- **REST API**: `/api/v1/` with full CRUD operations for tasks, executions, jobs, schedules, workers
- **GraphQL API**: `/graphql` with unified schema and introspection
- **Health Checks**: `/health`, `/ready`, `/live` for monitoring
- **Root Info**: `/` with service information and endpoint discovery

### Architecture Benefits
- **Modular Design**: Clean separation of concerns with dependency injection
- **Type Safety**: Unified types across REST and GraphQL APIs
- **Testability**: Service container pattern enables easy mocking
- **Configurability**: YAML/JSON configuration with CLI overrides
- **Observability**: Structured logging, tracing, and health checks

## Usage

### Basic Usage
```bash
# Run with defaults
cargo run --bin ratchet-server

# Custom configuration
cargo run --bin ratchet-server -- --config config.yaml

# CLI overrides
cargo run --bin ratchet-server -- --bind 127.0.0.1:8080 --database-url sqlite://custom.db
```

### Configuration
```bash
# Print default configuration
cargo run --bin ratchet-server -- --print-config > config.yaml

# Edit configuration and run
cargo run --bin ratchet-server -- --config config.yaml
```

### Development
```bash
# Enable GraphQL playground
cargo run --bin ratchet-server -- --playground

# Disable REST API
cargo run --bin ratchet-server -- --rest false

# Add task registry paths
cargo run --bin ratchet-server -- --registry-path ./tasks --registry-path ./examples
```

## Status

**Phase 4F Complete**: Server infrastructure and configuration system implemented.

**Current Limitations**: 
- GraphQL API temporarily disabled due to field mapping issues during migration
- Service implementations use placeholder bridge pattern to ratchet-lib
- Some axum router state management needs refinement

**Next Steps**:
- Fix GraphQL field mappings in ratchet-graphql-api
- Implement actual service bridges using ratchet-storage and other modular components
- Complete router state management for production deployment

## Configuration File Example

See `config-example.yaml` for a complete configuration example with all available options.

## Architecture

The server demonstrates successful extraction of:
- ✅ Unified API types (Phase 4A)
- ✅ Repository interfaces (Phase 4B) 
- ✅ Web middleware (Phase 4C)
- ✅ REST API implementation (Phase 4D)
- 🚧 GraphQL API implementation (Phase 4E - field mapping issues)
- ✅ Unified server setup (Phase 4F)

This validates the modular architecture approach and provides a foundation for production deployment.