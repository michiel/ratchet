# Changes

## v0.4.8 (2025-06-20)

### Major Features
- **Heartbeat System Enhancement**: Complete stdout output pipeline for heartbeat monitoring
  - Configured built-in heartbeat schedule to automatically send health check results to stdout
  - Added comprehensive stdio output destination support with configurable formatting
  - Implemented "[HEARTBEAT] " prefix with JSON formatting and metadata inclusion
  - Integrated OutputDeliveryManager throughout job processing pipeline

### API Enhancements
- **Extended Output Destination Support**: Added comprehensive stdio configuration options
  - New `UnifiedStdioConfig` with stream selection (stdout/stderr), formatting options, and metadata control
  - Enhanced `UnifiedOutputDestination` to support stdio alongside webhook, filesystem, and database destinations
  - Full OpenAPI documentation and schema support for all stdio configuration options

### Architecture Improvements
- **Complete Output Delivery Pipeline**: End-to-end output delivery from execution to destination
  - Integrated output delivery into job completion workflow with proper error handling
  - Enhanced job processor with OutputDeliveryManager for multi-destination support
  - Added schedule-to-job output destination inheritance for seamless configuration flow
  - Created comprehensive type conversion functions between API types and output manager types

### Infrastructure & System Improvements
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

### Bug Fixes & Stability
- **Compilation Error Resolution**: Fixed numerous build and test failures across the codebase
  - Resolved OpenAPI compilation issues with schema documentation
  - Fixed GraphQL mutation resolver errors with proper stdio field handling
  - Addressed missing dependency issues and import resolution problems
  - Enhanced test reliability with proper HTTP status codes and error handling

- **Cross-Platform Build Fixes**: Enhanced compatibility across target platforms
  - Improved Windows build compatibility with proper TLS configuration
  - Fixed macOS-specific dependency issues
  - Enhanced Linux distribution compatibility

### Documentation & Configuration
- **Comprehensive API Documentation**: Enhanced OpenAPI documentation for schedule webhooks and output destinations
  - Added detailed REST API payload examples with webhook integration scenarios
  - Created comprehensive example server configuration with all available options
  - Improved documentation for output destination configuration and usage

- **Maintainability Improvements**: Comprehensive maintainability enhancement plan
  - Detailed dependency analysis and reduction strategies
  - Code quality improvements and technical debt reduction
  - Enhanced documentation and architectural guidelines

### Developer Experience
- **Enhanced Testing Infrastructure**: Improved test reliability and coverage
  - Fixed e2e test failures with optimized test timeouts and proper error handling
  - Enhanced REST API test framework with better response handling
  - Implemented selective error sanitization for proper HTTP status code responses
  - Added comprehensive webhook integration testing with field-specific validation

This release represents a major infrastructure modernization focusing on performance, maintainability, and developer experience while adding robust output delivery capabilities and enhanced heartbeat monitoring with comprehensive stdout integration.

