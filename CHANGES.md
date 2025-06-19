# Changes

## v0.4.8 (2025-06-18)

### Major Improvements
- **Complete git2 to gitoxide Migration**: Fully migrated from legacy git2 library to modern gitoxide (gix) for all Git operations
  - Enhanced performance and reliability for Git repository access
  - Modern Rust implementation with better error handling
  - Reduced binary size and improved security posture

- **Comprehensive Dependency Modernization**: Major upgrade and consolidation of the entire dependency stack
  - **HTTP Stack Upgrade**: Updated to Axum 0.8, Tower 0.5, async-graphql 7.0 for latest features and performance
  - **Core Library Updates**: Tokio 1.45, SQLx 0.8, reqwest 0.12 with enhanced async capabilities
  - **System Libraries**: Updated nix to 0.30, getrandom to 0.3, and other core system dependencies
  - **Version Consolidation**: Eliminated duplicate dependency versions for base64, bitflags, HTTP stack components

- **Massive Dependency Reduction**: Removed 67 unused dependencies across 22 crates without functional changes
  - Reduced build times and binary size significantly
  - Improved compilation performance and reduced attack surface
  - Maintained full backward compatibility while streamlining the codebase

### Enhanced Developer Experience
- **Optimized Build Profiles**: Complete build profile restructuring for different use cases
  - **Developer Profile**: Made default with balanced optimization for development workflow
  - **Fast Development**: Maximum parallelism for rapid iteration cycles
  - **Distribution**: Fully optimized builds for production deployment
  - **Release**: Standard optimized builds with LTO and size optimization

- **Feature Flag Optimization**: Comprehensive feature flag system for conditional compilation
  - Modular build system allowing minimal, standard, complete, and developer configurations
  - Optional dependencies properly gated behind feature flags
  - Reduced compilation overhead for specific use cases

### Architecture & Infrastructure
- **HTTP Client Consolidation**: Unified HTTP functionality into ratchet-http crate
  - Centralized HTTP client management and configuration
  - Enhanced recording capabilities for debugging and testing
  - Optional server features for flexible deployment scenarios

- **MCP Server Enhancements**: Model Context Protocol server improvements
  - Full SSE (Server-Sent Events) support enabled in unified server mode
  - Enhanced stdio transport for CLI integration
  - Improved authentication and security context management

- **Cross-Platform Compatibility**: Enhanced support for Linux, macOS, and Windows
  - Hybrid TLS implementation: rustls for HTTP clients, OpenSSL for Git operations
  - Platform-specific optimizations and dependency management
  - Improved error handling across different operating systems

### Configuration & Documentation
- **Maintainability Improvements**: Comprehensive maintainability enhancement plan
  - Detailed dependency analysis and reduction strategies
  - Code quality improvements and technical debt reduction
  - Enhanced documentation and architectural guidelines

- **Build System Optimization**: Workspace-level dependency management improvements
  - Unified version management across all crates
  - Improved dependency resolution and conflict elimination
  - Enhanced build parallelism and compilation speed

### Bug Fixes & Stability
- **Compilation Error Resolution**: Fixed all dependency-related compilation issues
  - Restored necessary dependencies that were incorrectly identified as unused
  - Fixed feature flag configuration errors and import resolution
  - Resolved circular dependency issues in workspace configuration

- **Cross-Platform Build Fixes**: Enhanced compatibility across target platforms
  - Improved Windows build compatibility with proper TLS configuration
  - Fixed macOS-specific dependency issues
  - Enhanced Linux distribution compatibility

This release represents a major infrastructure modernization focusing on performance, maintainability, and developer experience while maintaining full backward compatibility and functionality.

## v0.4.7 (2025-06-17)

### Features
- **Scheduler Migration to tokio-cron-scheduler**: Complete migration from legacy polling-based scheduler to event-driven tokio-cron-scheduler architecture
  - **Phase 1**: Removed hardcoded heartbeat logic from scheduler core, implementing clean repository pattern abstraction
  - **Phase 2**: Full tokio-cron-scheduler integration with TokioCronSchedulerService implementation and job execution handlers
  - **Phase 3**: Service integration with REST APIs and SchedulerService trait interface
  - **Phase 4**: Configuration cleanup and registry-based task loading, eliminating hardcoded task references except for heartbeat initialization
- **Event-Driven Architecture**: Replaced 30-second polling intervals with sub-second precision cron-based execution
- **Repository Bridge Pattern**: Implemented RepositoryBridge for scheduler-to-database communication while maintaining clean separation of concerns
- **Heartbeat Schedule Management**: Registry-based heartbeat task initialization with automatic schedule creation only when heartbeat task exists
- **Enhanced Task Validation**: Added --fix flag for comprehensive task validation with automatic stub generation for missing components

### Bug Fixes
- **Scheduled Job Execution**: Fixed critical UUID/schedule ID mapping issue causing "Schedule not found" errors in heartbeat task execution
- **Query Parameter Support**: Resolved Refine.dev integration test failures with enhanced query parameter deserialization
- **Build Warnings**: Fixed critical compilation warnings from scheduler migration including variable scope and mutability issues
- **Test Compatibility**: Resolved cargo test errors in ratchet-web query_params_test with proper struct field initialization

### Developer Experience
- **Comprehensive Migration Documentation**: Added detailed 4-phase migration plan with technical specifications, rollback procedures, and progress tracking
- **E2E Testing**: Implemented comprehensive schedule workflow testing with adapter layer validation
- **Refine.dev Integration**: Complete REST API optimization with advanced filtering, sorting, and pagination support
- **Performance Improvements**: 90% resource reduction potential and 100+ concurrent job support through event-driven architecture

### Infrastructure
- **Authentication Integration**: Complete production authentication with session management and database integration
- **MCP Server Enhancement**: Re-enabled MCP server stdio functionality with full implementation and console command activation
- **Security Improvements**: Enhanced security testing infrastructure with comprehensive validation frameworks
- **Production Readiness**: Implemented HTTPS/TLS configuration and comprehensive monitoring/observability features

### Refactoring
- **Legacy Code Cleanup**: Removed outdated tests and source files as part of scheduler modernization
- **API Interface Unification**: Completed comprehensive filtering, CRUD operations, and MCP integration across all interfaces
- **GraphQL Enhancements**: Added missing CRUD mutations and comprehensive sorting support
- **Repository Pattern**: Eliminated repository bridge anti-patterns in favor of direct repository integration

This release represents a major architectural upgrade with the scheduler migration providing significant performance improvements, better reliability, and a foundation for advanced scheduling features while maintaining full backward compatibility.