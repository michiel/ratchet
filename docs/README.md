# Ratchet Documentation Index

## Core Documentation

### [Architecture Guide](ARCHITECTURE.md)
Comprehensive technical documentation covering:
- System architecture overview
- Process separation design
- Component responsibilities
- Technology stack
- Implementation details

### [REST API Documentation](REST_API_README.md)
Complete REST API reference including:
- Refine.dev compatibility
- Endpoint documentation
- Error handling patterns
- Pagination and filtering
- Integration examples

### [OpenAPI Specification](openapi.yaml)
Machine-readable API specification with:
- Complete endpoint definitions
- Request/response schemas
- Authentication details (planned)
- Interactive viewer: [openapi-viewer.html](openapi-viewer.html)

### [Fetch API Guide](FETCH_API.md)
JavaScript fetch API documentation for task developers:
- HTTP request examples
- Request/response handling
- Error handling patterns
- Limitations and considerations

### [Cross-Platform Considerations](CROSS-PLATFORM-CONSIDERATIONS.md)
Platform-specific deployment guide covering:
- Windows compatibility
- macOS considerations
- Linux deployment
- File system differences
- Performance optimizations

## Planning Documents

### [DAG Workflow Engine Plan](plans/DAG_WORKFLOW_PLAN.md)
Comprehensive plan for implementing visual-editor-ready DAG workflows with:
- Branching logic and conditional execution
- Parallel task execution
- Visual layout support for future editor
- State management and resumability

### [Execution Module Restructure Plan](plans/EXECUTION_RESTRUCTURE_PLAN.md)
Future improvement plan for the execution module architecture (not yet implemented).

## Additional Resources

### Shell Scripts
- [rest-api-examples.sh](rest-api-examples.sh) - Example REST API calls using curl

### Main Project Documentation
- [README.md](../README.md) - Project overview and quick start guide
- [TODO.md](../TODO.md) - Comprehensive architectural roadmap
- [CHANGES.md](../CHANGES.md) - Release notes and changelog
- [CLI-SERVE.md](../CLI-SERVE.md) - Server command documentation
- [example-config.yaml](../example-config.yaml) - Complete configuration example