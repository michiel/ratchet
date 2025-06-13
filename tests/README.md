# Ratchet Integration Tests

This directory contains comprehensive end-to-end integration tests for the Ratchet task execution platform.

## Test Overview

### `ratchet_serve_e2e_test.rs`

A comprehensive end-to-end test that validates the complete `ratchet serve` workflow:

#### Test Scenario: `test_ratchet_serve_end_to_end_workflow`

**What it tests:**
1. **Repository Loading**: Loads tasks from a filesystem repository (sample JS tasks)
2. **Server Startup**: Starts a full ratchet server with REST, GraphQL, and MCP APIs
3. **GraphQL API Coverage**: Tests all major GraphQL queries and mutations
4. **Task Discovery**: Queries available tasks through the GraphQL API
5. **Task Execution**: Schedules task execution with input parameters
6. **Webhook Integration**: Configures webhook output destinations for results
7. **Result Verification**: Listens for webhook delivery and verifies expected results

**Step-by-step workflow:**
1. 📡 Start test webhook server to receive execution results
2. 📁 Set up test repository with sample tasks (Addition Task)
3. ⚙️ Create comprehensive test configuration (database, server, APIs, registries)
4. 🌐 Start ratchet server with full API stack
5. 🔌 Connect to GraphQL API and verify health
6. 🔍 Query for available tasks and verify repository sync
7. 🔬 Get detailed task information including schemas
8. ⚡ Schedule task execution with webhook output destination
9. 📈 Monitor execution status and job queue
10. 📋 Check job queue for created jobs
11. ⏳ Wait for webhook delivery (with timeout)
12. ✅ Verify webhook payload contains expected results
13. 📊 Get final system statistics

**GraphQL API Coverage:**
- Health checks (`health` query)
- Task listing (`tasks` query)
- Task details (`task` query by ID)
- Task execution (`executeTask` mutation)
- Execution monitoring (`executions` query)
- Job queue monitoring (`jobs` query)
- System statistics (`taskStats`, `executionStats`, `jobStats`)

**Integration Points Tested:**
- Database initialization and migrations
- Repository synchronization
- GraphQL schema and resolvers
- Job queue management
- Webhook configuration and delivery
- Output destination system
- Error handling and timeouts

#### Test Scenario: `test_graphql_playground_queries_compatibility`

**What it tests:**
- Validates that all GraphQL Playground queries work correctly
- Ensures schema compatibility for all predefined queries
- Tests query structure without requiring full execution pipeline

**Queries tested:**
- List All Tasks
- Task Executions (with filters)
- Task Statistics
- Jobs Queue (with status filters)

## Expected Behavior

### In Test Environment
- The test creates an isolated environment with in-memory database
- Server runs on random ports to avoid conflicts
- Repository loading works from the sample tasks directory
- GraphQL API responds correctly to all queries
- Job creation and scheduling works
- Webhook configuration is properly stored

### Potential Limitations
- Full task execution may not work without worker processes
- Webhook delivery depends on the execution pipeline being active
- Some operations may return runtime errors while maintaining schema compatibility

### Success Criteria
- ✅ All GraphQL queries execute without schema errors
- ✅ Tasks are loaded from repository and available via API
- ✅ Jobs can be created with webhook output destinations
- ✅ Server starts and responds to health checks
- ✅ System statistics are accessible
- ✅ Webhook payloads are received (when execution completes)

## Running the Tests

### Prerequisites
```bash
# Ensure sample tasks exist
ls sample/js-tasks/tasks/addition/

# Build the project
cargo build
```

### Run Integration Tests
```bash
# Run all integration tests
cargo test --test ratchet_serve_e2e_test

# Run with output
cargo test --test ratchet_serve_e2e_test -- --nocapture

# Run specific test
cargo test --test ratchet_serve_e2e_test test_ratchet_serve_end_to_end_workflow -- --nocapture
```

### Debug Mode
```bash
# Run with debug logging
RUST_LOG=debug cargo test --test ratchet_serve_e2e_test -- --nocapture
```

## Test Configuration

The test creates a comprehensive configuration covering:

### Database
- SQLite in-memory database for isolation
- Automatic migrations
- Foreign key constraints enabled

### Server
- Random port allocation to avoid conflicts
- CORS enabled for cross-origin requests
- Request ID and tracing enabled
- GraphQL playground enabled for debugging

### APIs
- REST API at `/api/v1/*`
- GraphQL API at `/graphql`
- GraphQL Playground at `/playground`
- Health endpoints at `/health`

### Repository
- Filesystem repository pointing to `sample/js-tasks`
- Auto-sync enabled
- Task validation enabled
- Addition task with schemas and test cases

### Output Destinations
- Webhook destination with test server
- Retry policy with exponential backoff
- JSON content type
- 30-second timeout

## Extending the Tests

### Adding New Test Scenarios
1. Create new test function in `ratchet_serve_e2e_test.rs`
2. Use existing helper functions for server setup
3. Add specific GraphQL queries or REST API calls
4. Verify expected behavior

### Testing Additional Tasks
1. Add task directories to `sample/js-tasks/tasks/`
2. Ensure proper metadata.json and schema files
3. Reference task in test by name or UUID
4. Test task-specific input/output validation

### Testing Different Output Destinations
1. Modify the `outputDestinations` configuration in execute mutations
2. Add corresponding test servers (filesystem, etc.)
3. Verify delivery to multiple destinations

### Performance Testing
1. Add timing measurements around critical operations
2. Test with larger numbers of tasks or executions
3. Monitor memory usage and cleanup

## Integration with CI/CD

These tests are designed to run in continuous integration environments:

- **Fast startup**: Uses in-memory database and random ports
- **Isolated**: No external dependencies or persistent state
- **Comprehensive**: Covers major integration points
- **Timeout-aware**: All network operations have reasonable timeouts
- **Error-tolerant**: Distinguishes between schema errors and runtime limitations

## Troubleshooting

### Common Issues

**Port conflicts:**
- Tests use random ports, but ensure no other services are binding to all interfaces

**Missing sample tasks:**
- Verify `sample/js-tasks/tasks/addition/` exists with required files
- Check metadata.json, input.schema.json, output.schema.json, main.js

**Timeout issues:**
- Increase timeouts in test configuration
- Check server startup logs for initialization delays

**Schema errors:**
- Review GraphQL schema changes
- Update test queries to match current schema
- Check resolver implementations

**Database migration errors:**
- Ensure migration files are included in the build
- Check SeaORM entity definitions match migration schemas

### Debug Output

The test includes extensive debug output:
- 📡 Webhook server events
- 📁 Repository loading status
- ⚙️ Configuration details
- 🌐 Server startup progress
- 🔍 Query results and responses
- ⚡ Execution scheduling
- 📊 System statistics

Use `--nocapture` flag to see all debug output during test execution.