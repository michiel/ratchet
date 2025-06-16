# Ratchet System Architecture

## Overview

Ratchet is a production-ready task automation and execution platform built in Rust. The system follows a modular architecture with 15 specialized crates, providing REST API, GraphQL API, and Model Context Protocol (MCP) interfaces for comprehensive task management.

## High-Level Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                     Client Interfaces                          │
├─────────────────┬─────────────────┬─────────────────────────────┤
│   REST API      │   GraphQL API   │       MCP Protocol         │
│   (HTTP/JSON)   │   (HTTP/JSON)   │   (JSON-RPC over stdio/SSE) │
├─────────────────┼─────────────────┼─────────────────────────────┤
│                      API Layer                                 │
│   ratchet-rest-api  │  ratchet-graphql-api  │  ratchet-mcp   │
├─────────────────────────────────────────────────────────────────┤
│                     Service Layer                              │
│            ratchet-lib (Business Logic Core)                   │
├─────────────────────────────────────────────────────────────────┤
│                 Infrastructure Layer                           │
├─────────────────┬─────────────────┬─────────────────────────────┤
│ ratchet-storage │ ratchet-execution│     ratchet-core           │
│ ratchet-http    │ ratchet-js      │     ratchet-config         │
│ ratchet-logging │ ratchet-runtime │     ratchet-caching        │
└─────────────────┴─────────────────┴─────────────────────────────┘
```

## Crate Architecture

### Core Foundation
- **`ratchet-core`**: Domain models, types, and service registry
- **`ratchet-lib`**: Primary business logic and proven API implementations
- **`ratchet-cli`**: Command-line interface and interactive console

### API Layer
- **`ratchet-rest-api`**: REST API endpoints with OpenAPI documentation
- **`ratchet-graphql-api`**: GraphQL schema with subscriptions support
- **`ratchet-mcp`**: Model Context Protocol server with dual transport

### Execution Infrastructure
- **`ratchet-execution`**: Process execution and worker management
- **`ratchet-js`**: JavaScript task execution with Boa engine
- **`ratchet-runtime`**: Alternative execution patterns

### Data & Communication
- **`ratchet-storage`**: Repository pattern with Sea-ORM integration
- **`ratchet-http`**: HTTP client with mock support
- **`ratchet-ipc`**: Inter-process communication abstractions

### System Infrastructure
- **`ratchet-config`**: Configuration management with environment support
- **`ratchet-logging`**: Structured logging with LLM integration
- **`ratchet-caching`**: Multiple cache backends (in-memory, LRU, TTL)

### Extensibility
- **`ratchet-plugin`**: Plugin infrastructure and lifecycle management
- **`ratchet-resilience`**: Circuit breakers, retry policies, graceful shutdown

## Key Architectural Patterns

### 1. Repository Pattern
All data access goes through repository interfaces defined in `ratchet-storage`, providing:
- Database abstraction with Sea-ORM
- Transaction management
- Query optimization
- Mock implementations for testing

### 2. Service Registry
Dependency injection system in `ratchet-core` enables:
- Testable service composition
- Runtime service discovery
- Async-aware dependency resolution

### 3. Type Safety
- Newtype pattern for IDs (`TaskId`, `ExecutionId`, `JobId`)
- Builder patterns for complex object construction
- Comprehensive error types with context

### 4. Configuration Management
Domain-specific configuration with:
- Environment variable support (`RATCHET_*` prefix)
- YAML/JSON configuration files
- Validation and default values

## API Architecture

### REST API (`ratchet-rest-api`)
- **Endpoints**: Tasks, executions, jobs, schedules, workers
- **Features**: Pagination, filtering, validation, retry logic
- **Security**: JWT authentication, API keys, rate limiting
- **Documentation**: OpenAPI 3.0 specification

### GraphQL API (`ratchet-graphql-api`)
- **Schema**: Type-safe queries and mutations
- **Real-time**: Subscription support with event broadcasting
- **Optimization**: DataLoader pattern for N+1 query prevention
- **Introspection**: Configurable for development/production

### MCP Protocol (`ratchet-mcp`)
- **Transport**: stdio for CLI, SSE for HTTP clients
- **Tools**: 6 production tools (execute_task, list_tasks, get_status, etc.)
- **Features**: Batch processing, progress notifications, streaming
- **Integration**: Claude Desktop and LLM client support

## Security Architecture

### Authentication & Authorization
- **JWT Tokens**: Role-based access control (RBAC)
- **API Keys**: Multi-method extraction with permissions
- **Session Management**: Timeout enforcement and cleanup

### Security Testing Infrastructure
Comprehensive security testing framework across all API layers:

#### REST API Security Tests (`ratchet-rest-api/tests/security_tests.rs`)
- Authentication security (JWT, API keys, unauthenticated access)
- Authorization testing (RBAC, privilege escalation prevention)
- Input validation (SQL injection, XSS, request limits, JSON bombs)
- Rate limiting and security headers validation
- Session security and fixation protection

#### GraphQL Security Tests (`ratchet-graphql-api/tests/security_tests.rs`)
- GraphQL-specific authentication and authorization
- Query complexity and depth limit validation
- Introspection security for production environments
- Batch query abuse prevention
- Field-level authorization security

#### MCP Protocol Security Tests (`ratchet-mcp/tests/security_tests.rs`)
- Message validation and protocol integrity
- Resource protection and connection security
- Data integrity and corruption handling
- Authentication and session security
- Protocol violation detection

#### Security Features
- **Vulnerability Assessment**: Automated security scoring with severity classification
- **Security Reporting**: Comprehensive reports with actionable recommendations
- **Threat Modeling**: Real-world security scenarios and attack simulations
- **Production Ready**: Security testing integrated into CI/CD pipeline

### Security Headers & Middleware
- HSTS, CSP, X-Frame-Options, X-Content-Type-Options
- Request size limits and content validation
- CORS configuration for cross-origin requests

## Data Architecture

### Database Layer
- **Primary**: SQLite with Sea-ORM migrations
- **Repositories**: Type-safe data access patterns
- **Transactions**: ACID compliance for complex operations

### Caching Strategy
- **Multiple Backends**: In-memory, LRU, TTL, Moka
- **Use Cases**: Task definitions, execution results, configuration
- **Invalidation**: Event-driven cache updates

### Output Destinations
- **Filesystem**: Atomic writes with multiple formats (JSON, YAML, CSV)
- **Webhooks**: HTTP delivery with authentication and retry logic
- **Template Engine**: Dynamic path/URL generation

## Execution Architecture

### Task Execution
- **Process Isolation**: Secure execution in separate processes
- **JavaScript Support**: Boa engine for JS task execution
- **Worker Management**: Distributed execution with health monitoring

### Job Scheduling
- **Priority Queue**: Job scheduling with retry logic
- **Cron Integration**: Schedule-based task execution
- **Batch Processing**: Efficient bulk operation handling

## Testing Architecture

### Unit Testing
- **Coverage**: 486+ tests across entire workspace
- **Mocking**: HTTP client mocks, repository mocks
- **Isolation**: Tests run independently with clean state

### Integration Testing
- **API Testing**: Complete REST and GraphQL endpoint validation
- **MCP Testing**: Protocol compliance and tool functionality
- **Performance Testing**: Load testing and latency measurement

### Security Testing
- **Comprehensive Coverage**: Security tests for all three API layers
- **Automated Assessment**: Vulnerability scoring and classification
- **Simulation Framework**: Mock-based security testing for consistency

## Performance Architecture

### Optimization Strategies
- **Pure Rust TLS**: rustls for better performance and security
- **Async Runtime**: Tokio for high-concurrency operations
- **Connection Pooling**: Database connection management
- **Streaming**: Real-time data delivery for large responses

### Monitoring & Observability
- **Structured Logging**: JSON logs with contextual enrichment
- **Error Pattern Recognition**: AI-optimized error analysis
- **Performance Metrics**: Latency, throughput, and resource usage tracking

## Deployment Architecture

### Build Configuration
- **Feature Flags**: Conditional compilation for different environments
- **Static Builds**: Self-contained binaries for deployment
- **Cross-Platform**: Linux, macOS, Windows support

### Configuration Management
- **Environment Variables**: Runtime configuration overrides
- **Configuration Files**: YAML/JSON with validation
- **Default Values**: Sensible defaults for all settings

## Future Architecture Considerations

### Scalability
- Microservice decomposition paths
- Distributed execution clustering
- Database sharding strategies

### Extensibility
- Plugin system expansion
- Custom protocol adapters
- Third-party integrations

### Cloud-Native Features
- Kubernetes deployment patterns
- Container orchestration
- Service mesh integration

## Migration Status

The system has successfully completed a major architectural migration:
- **Phase 1**: ✅ Foundation (15-crate modular architecture)
- **Phase 2**: ✅ API Implementation (REST, GraphQL, MCP)
- **Phase 3**: ✅ Security Infrastructure (comprehensive testing)
- **Current**: Production-ready with ongoing optimization

For detailed migration history, see `docs/ARCHITECTURE_MIGRATION_STATUS.md`.