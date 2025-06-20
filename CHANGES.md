# Changes

## v0.4.9 (2025-06-20)

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

### Bug Fixes & Stability
- **Compilation Error Resolution**: Fixed numerous build and test failures across the codebase
  - Resolved OpenAPI compilation issues with schema documentation
  - Fixed GraphQL mutation resolver errors with proper stdio field handling
  - Addressed missing dependency issues and import resolution problems
  - Enhanced test reliability with proper HTTP status codes and error handling

### Documentation & Configuration
- **Comprehensive API Documentation**: Enhanced OpenAPI documentation for schedule webhooks and output destinations
  - Added detailed REST API payload examples with webhook integration scenarios
  - Created comprehensive example server configuration with all available options
  - Improved documentation for output destination configuration and usage

### Developer Experience
- **Enhanced Testing Infrastructure**: Improved test reliability and coverage
  - Fixed e2e test failures with optimized test timeouts and proper error handling
  - Enhanced REST API test framework with better response handling
  - Implemented selective error sanitization for proper HTTP status code responses
  - Added comprehensive webhook integration testing with field-specific validation

This release focuses on robust output delivery capabilities, enhanced heartbeat monitoring, and improved system reliability with comprehensive stdout integration for health monitoring workflows.

