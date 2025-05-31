# Ratchet Testing Infrastructure

This document describes the comprehensive testing infrastructure for the Ratchet project, including unit tests, integration tests, and end-to-end tests.

## Overview

The testing infrastructure covers all major components of Ratchet:
- REST API endpoints
- Load Balancer strategies
- Circuit Breaker behavior
- Complete task execution flow
- Database operations
- Worker process management

## Test Categories

### 1. Unit Tests

Located in the source files next to the code they test, unit tests focus on individual functions and modules.

Run unit tests:
```bash
cargo test --lib
```

### 2. Integration Tests

Located in `ratchet-lib/tests/`, integration tests verify component interactions.

#### REST API Tests (`rest_api_comprehensive_test.rs`)
- **CRUD Operations**: Tests for Tasks, Jobs, Schedules, Executions
- **Pagination**: Query parameter handling with `_start`, `_end`
- **Filtering**: Status and resource-specific filters
- **Error Handling**: 404, 400, 422 error responses
- **Concurrent Requests**: Thread safety verification
- **Health Monitoring**: Health and stats endpoints

Run REST API tests:
```bash
cargo test --test rest_api_comprehensive_test
```

#### Load Balancer Tests (`load_balancer_test.rs`)
- **Round Robin**: Even distribution across workers
- **Least Loaded**: Selection based on current load
- **Weighted Round Robin**: Distribution based on weights
- **Health Monitoring**: Unhealthy worker exclusion
- **Performance Metrics**: Aggregate statistics
- **Concurrent Operations**: Thread-safe worker selection

Run load balancer tests:
```bash
cargo test --test load_balancer_test
```

#### Circuit Breaker Tests (`circuit_breaker_test.rs`)
- **State Transitions**: Closed → Open → Half-Open → Closed
- **Failure Thresholds**: Automatic circuit opening
- **Recovery**: Success threshold for closing
- **Retry Integration**: Circuit breaker with retry policies
- **Backoff Strategies**: Fixed, Linear, Exponential
- **Jitter**: Random delay variation
- **Concurrent Access**: Thread-safe state management

Run circuit breaker tests:
```bash
cargo test --test circuit_breaker_test
```

### 3. End-to-End Tests (`task_execution_e2e_test.rs`)

Complete workflow tests covering the entire task execution pipeline:
- **Simple Execution**: Basic task execution flow
- **Validation Errors**: Schema validation failures
- **Runtime Errors**: JavaScript execution errors
- **Concurrent Executions**: Multiple tasks in parallel
- **Job Queue**: Priority-based job processing
- **HTTP Requests**: Tasks with external API calls
- **Worker Recovery**: Failure handling and recovery
- **Timeouts**: Task execution timeouts

Run E2E tests:
```bash
cargo test --test task_execution_e2e_test
```

## Running All Tests

Run the complete test suite:
```bash
# All tests
cargo test --all

# With output for debugging
cargo test --all -- --nocapture

# Specific test pattern
cargo test --all -- test_circuit_breaker

# With test threads limited (for debugging)
cargo test --all -- --test-threads=1
```

## Test Coverage

Generate test coverage report:
```bash
# Install tarpaulin
cargo install cargo-tarpaulin

# Generate coverage report
cargo tarpaulin --out Html --output-dir coverage
```

## Performance Testing

For load testing the REST API:
```bash
# Install Apache Bench
sudo apt-get install apache2-utils

# Simple load test
ab -n 1000 -c 10 http://localhost:8000/health

# POST request load test
ab -n 100 -c 5 -p task.json -T application/json http://localhost:8000/tasks
```

## Debugging Tests

### Enable Logging
```bash
# Set log level for tests
RUST_LOG=debug cargo test --test rest_api_comprehensive_test -- --nocapture
```

### Single Test Execution
```bash
# Run specific test function
cargo test test_circuit_breaker_state_transitions -- --exact
```

### Test Database
Tests use in-memory SQLite by default. To use a file-based database for debugging:
```rust
let db_config = DatabaseConfig {
    url: "sqlite:test.db".to_string(), // Instead of sqlite::memory:
    // ...
};
```

## Writing New Tests

### Test Helpers

Use provided test helpers for common setup:
```rust
// REST API test server
async fn create_full_test_server() -> (TestServer, RepositoryFactory)

// Task creation
async fn create_test_task(name: &str, content: &str) -> PathBuf

// Execution environment
async fn setup_execution_environment() -> (Arc<ProcessTaskExecutor>, RepositoryFactory)
```

### Best Practices

1. **Isolation**: Each test should be independent
2. **Cleanup**: Use `defer` or cleanup functions
3. **Assertions**: Use descriptive assertion messages
4. **Timeouts**: Set reasonable timeouts for async tests
5. **Mocking**: Use mock implementations for external dependencies

### Example Test Structure
```rust
#[tokio::test]
async fn test_feature_behavior() {
    // Arrange
    let (server, repos) = create_full_test_server().await;
    
    // Act
    let response = server.post("/endpoint").json(&data).await;
    
    // Assert
    assert_eq!(response.status_code(), StatusCode::OK);
    let body: Value = response.json();
    assert_eq!(body["field"], expected_value);
}
```

## Continuous Integration

Tests are automatically run on:
- Pull requests
- Commits to main branch
- Release tags

See `.github/workflows/` for CI configuration.

## Troubleshooting

### Common Issues

1. **Port Already in Use**
   - Tests use random ports, but conflicts can occur
   - Solution: Kill processes using the port

2. **Database Lock Errors**
   - SQLite in-memory databases are isolated per connection
   - Solution: Ensure proper connection handling

3. **Timeout Failures**
   - Default timeouts may be too short for slower systems
   - Solution: Increase timeout values in tests

4. **Worker Process Failures**
   - Worker processes may fail to spawn
   - Solution: Check system resources and permissions

## Future Improvements

1. **Property-Based Testing**: Add proptest for edge cases
2. **Benchmark Suite**: Performance regression tests
3. **Chaos Testing**: Simulate network failures and crashes
4. **Visual Testing**: UI component testing for GraphQL playground
5. **Contract Testing**: API contract verification