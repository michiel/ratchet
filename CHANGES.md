# Changes

## Version 0.4.3 (2025-06-15)

### Bug Fixes
- **GraphQL Context Extension Fix**: Resolved critical GraphQL context extension issues preventing API operations
- **Axum 0.7 Compatibility**: Fixed GraphQL handler compatibility with Axum 0.7 framework upgrade
- **Test Infrastructure**: Enhanced integration test reliability with comprehensive server readiness checks
- **Error Handling**: Improved GraphQL client error handling with proper HTTP status code validation

### Features  
- **OpenAPI 3.0 Documentation**: Added comprehensive interactive API documentation with utoipa integration
- **Enhanced MCP Server**: Improved Model Context Protocol server robustness for Claude Code compatibility
- **Input Validation**: Implemented comprehensive input validation and error sanitization across all endpoints
- **Performance Optimization**: Reduced end-to-end test execution time from 35s to 2.64s

### Developer Experience
- **Security Improvements**: Added ErrorSanitizer for secure error handling and preventing information leakage
- **REST API Testing**: Implemented comprehensive testing framework with full endpoint coverage
- **Documentation Updates**: Enhanced LLM integration docs with quick start guides and troubleshooting
- **Build Reliability**: Resolved all cargo build and test compilation errors across workspace

### Infrastructure
- **Dependency Upgrades**: Successfully upgraded Axum from 0.6 to 0.7 with full compatibility maintained
- **Windows Support**: Fixed PowerShell installation script property access errors
- **Cross-Platform**: Improved build reliability across Linux, macOS, and Windows platforms

This release focuses on stability and developer experience improvements, resolving critical GraphQL functionality issues while adding comprehensive API documentation and enhanced testing infrastructure.