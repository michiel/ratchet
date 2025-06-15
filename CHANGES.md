# Changes

## v0.4.6 (2025-06-15)

### Bug Fixes
- **Repository Synchronization**: Fixed critical issue where repository sync claimed success but tasks weren't persisted to database, causing empty API responses
- **Database Persistence**: Resolved sync process using stub repository implementations instead of actual SeaORM repositories for data persistence
- **API Data Access**: Fixed disconnect between successful sync logging and empty GraphQL/REST API responses by ensuring consistent repository layer usage

### Infrastructure
- **Docker Support**: Added comprehensive Docker containerization with multi-stage builds, GitHub Container Registry CI/CD pipeline, and multi-platform support (AMD64/ARM64)
- **Container Security**: Implemented security-focused container with non-root user, health checks, and proper volume mounts for data persistence
- **CI/CD Pipeline**: Added automated Docker image publishing to ghcr.io on releases with semantic versioning and build attestation
- **Development Tools**: Added Docker Compose setup for local development and .dockerignore for optimized builds

### Developer Experience
- **Sync Verification**: Added database verification step to confirm actual task persistence during repository synchronization
- **Error Diagnostics**: Enhanced sync logging to distinguish between claimed success and actual database persistence
- **Repository Consistency**: Ensured sync process and API layer use compatible repository implementations for data access

This release resolves the critical issue where users experienced empty API responses despite successful repository synchronization, and adds comprehensive Docker support for containerized deployments.

## v0.4.5 (2025-06-15)

### Features
- **Complete API Interface Implementation**: Implemented fully functional CRUD operations across GraphQL, REST, and MCP interfaces with repository integration
- **GraphQL Mutations**: Added complete working mutations for all entities (tasks, executions, jobs, schedules) with proper validation and database persistence
- **REST API Enhancement**: Completed fully functional CRUD operations with consistent error handling, input validation, and repository integration across all endpoints
- **MCP Pagination Tools**: Enhanced MCP interface with comprehensive pagination support including new working tools: `ratchet_list_executions`, `ratchet_list_jobs`, `ratchet_list_schedules`
- **Cross-API Consistency**: Standardized error handling patterns using InputValidator and ErrorSanitizer across all three API interfaces

### Bug Fixes
- **Type System Compatibility**: Fixed GraphQL type conversion issues between GraphQLApiId and repository interfaces
- **Enum Variant Corrections**: Resolved ExecutionStatus enum usage (Pending vs Queued) across all API interfaces
- **MCP Test Compatibility**: Updated MCP e2e tests to handle new paginated response structure from enhanced list tools
- **Compilation Errors**: Fixed various type casting issues and Option<String> handling across REST and GraphQL implementations

### Developer Experience
- **Unified Type System**: Leveraged UnifiedTask, UnifiedExecution, UnifiedJob, and UnifiedSchedule types for consistent API behavior
- **Comprehensive Validation**: Added proper input validation for security and data integrity across all API endpoints
- **Repository Pattern Integration**: Connected all API stubs to repository layer for proper data persistence and retrieval
- **Error Sanitization**: Implemented secure error handling to prevent information leakage in API responses

### Infrastructure
- **Test Coverage**: All tests now pass including updated MCP e2e tests for new pagination functionality
- **Code Reuse**: Implemented proper repository pattern integration for consistent behavior across API interfaces
- **Response Formatting**: Standardized API response structures with proper metadata and pagination information
- **Cross-Platform Compatibility**: Ensured all API implementations work correctly across Linux, macOS, and Windows

This implementation provides complete feature parity across all three API interfaces, enabling users to perform full CRUD operations through GraphQL mutations, REST endpoints, or MCP tools with consistent behavior and validation.

## Version 0.4.4 (2025-06-15)

### Features
- **Unified MCP Command Structure**: Added new `mcp` command for general-purpose MCP server operations with SSE transport as default
- **Claude Code Compatibility**: Enhanced MCP protocol support with "2025-03-26" version for full Claude Code integration
- **Command Harmonization**: Standardized `mcp` and `mcp-serve` commands to share identical configuration and behavior, differing only in transport defaults

### Bug Fixes
- **MCP Protocol Version Support**: Fixed protocol handshake issues with Claude Code by adding latest protocol version "2025-03-26"
- **Future Type Compatibility**: Resolved Rust compilation errors in MCP server transport handling using boxed futures
- **Claude Code Tool Name Validation**: Fixed MCP tool name validation errors by replacing dots with underscores in all tool names to comply with Claude Code's `^[a-zA-Z0-9_-]{1,64}$` pattern

### Developer Experience
- **Comprehensive Documentation Updates**: Updated README.md, MCP documentation, and integration guides to reflect new command structure
- **Claude Code Integration**: Added seamless integration with `claude mcp add` command for zero-configuration setup
- **Flexible Transport Options**: Both MCP commands now support both stdio and SSE transports via `--transport` flag

### Infrastructure
- **Command Refactoring**: Consolidated MCP server logic into shared `mcp_command_with_config` function for better maintainability
- **Transport Detection**: Added intelligent stdio mode detection for proper logging behavior
- **Configuration Consistency**: Ensured both MCP commands use identical defaults and configuration handling

This release focuses on Claude Code integration and command structure improvements, providing a unified and flexible MCP server experience for both local desktop and web application use cases.