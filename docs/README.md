# Ratchet Documentation Index

## Core Documentation

### [Architecture Guide](ARCHITECTURE.md)
Comprehensive technical documentation covering:
- System architecture overview
- Process separation design
- Component responsibilities
- Technology stack
- Implementation details

### [CLI Usage Guide](CLI_USAGE.md)
Complete command-line interface documentation:
- Server command usage and configuration
- Environment variables and config files
- Available endpoints and features
- GraphQL examples and session workflows

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

### [Output Destinations Guide](OUTPUT_DESTINATIONS.md)
Flexible output delivery system for task results:
- Filesystem, webhook, database, and S3 destinations
- Template variables and dynamic paths
- Authentication and retry policies
- REST API and GraphQL usage examples
- Configuration and best practices

### [Logging System](LOGGING_OVERVIEW.md)
Comprehensive logging infrastructure with AI-powered error analysis:
- [System Overview](LOGGING_OVERVIEW.md) - Architecture and components
- [Usage Guide](LOGGING_USAGE.md) - Implementation examples and best practices
- Structured logging with contextual enrichment
- Error pattern recognition and automated analysis
- LLM-optimized export formats for AI debugging

## Planning Documents

### [DAG Workflow Engine Plan](plans/DAG_WORKFLOW_PLAN.md)
Comprehensive plan for implementing visual-editor-ready DAG workflows with:
- Branching logic and conditional execution
- Parallel task execution
- Visual layout support for future editor
- State management and resumability

### [Task & Workflow Marketplace Plan](plans/TASK_MARKETPLACE_PLAN.md)
Complete ecosystem design for bundling, distributing, and monetizing tasks:
- Bundle packaging format with dependencies
- Public and private registries
- Marketplace with discovery, ratings, and payments
- Enterprise features and compliance
- Security model with code signing

### [Execution Module Restructure Plan](plans/EXECUTION_RESTRUCTURE_PLAN.md)
Future improvement plan for the execution module architecture (not yet implemented).

## Additional Resources

### Shell Scripts
- [rest-api-examples.sh](rest-api-examples.sh) - Example REST API calls using curl

### Main Project Documentation
- [README.md](../README.md) - Project overview and quick start guide
- [TODO.md](../TODO.md) - Comprehensive architectural roadmap
- [CHANGES.md](../CHANGES.md) - Release notes and changelog
- [CLI Usage Guide](CLI_USAGE.md) - Server command documentation
- [example-config.yaml](../example-config.yaml) - Complete configuration example