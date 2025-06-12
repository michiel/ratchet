# Legacy Test Archive

## Overview

This directory contains the archived integration tests from `ratchet-lib/tests/` that were removed during the legacy deprecation process (Phase 8.5).

## Why These Tests Were Archived

These tests were deeply integrated with the legacy `ratchet_lib::` architecture that was completely removed in Phase 4 of the legacy deprecation plan. They relied on:

- **Legacy Database Layer**: `ratchet_lib::database` (removed - replaced by ratchet-storage)
- **Legacy Configuration**: `ratchet_lib::config` (replaced by ratchet-config)
- **Legacy Execution**: `ratchet_lib::execution` (moved to ratchet-execution)
- **Legacy REST API**: `ratchet_lib::rest` (moved to ratchet-rest-api)
- **Legacy Output System**: `ratchet_lib::output` (moved to ratchet-output)

## Modern Test Coverage

The functionality tested by these legacy tests is now covered by modern test suites in the modular crates:

### REST API Testing
- **Modern Location**: `ratchet-rest-api/tests/` and `ratchet-server/tests/`
- **Legacy Tests Replaced**:
  - `enhanced_rest_api_test.rs`
  - `rest_api_test.rs`
  - `output_destinations_rest_api_test.rs`

### GraphQL Testing
- **Modern Location**: `ratchet-graphql-api/tests/` and `ratchet-server/tests/`
- **Legacy Tests Replaced**:
  - `graphql_playground_queries_test.rs`
  - `output_destinations_graphql_test.rs`

### Storage and Database Testing
- **Modern Location**: `ratchet-storage/tests/`
- **Legacy Tests Replaced**: All tests using `ratchet_lib::database`

### Execution Testing
- **Modern Location**: `ratchet-execution/tests/` and `ratchet-runtime/tests/`
- **Legacy Tests Replaced**:
  - `task_execution_e2e_test.rs`
  - `process_separation_integration_test.rs`
  - `validate_test.rs`

### Output and Delivery Testing
- **Modern Location**: `ratchet-output/tests/`
- **Legacy Tests Replaced**:
  - `output_delivery_integration_test.rs`
  - `output_config_validation_test.rs`
  - `addition_task_webhook_integration_test.rs`

### Infrastructure Testing
- **Modern Location**: `ratchet-resilience/tests/` and `ratchet-execution/tests/`
- **Legacy Tests Replaced**:
  - `circuit_breaker_test.rs`
  - `load_balancer_test.rs`

## Archive Date

These tests were archived on December 2024 during Phase 8.5 of the legacy deprecation plan.

## Migration Notes

Rather than attempting to migrate these tests (which would require significant rewriting due to the architectural changes), the decision was made to:

1. **Archive these legacy tests** for historical reference
2. **Rely on modern test coverage** in the modular crates
3. **Focus effort on completing** the ratchet-lib elimination

The modern modular architecture provides better test coverage with:
- **Isolated testing** per crate
- **Modern test infrastructure** with proper mocking and fixtures
- **Faster test execution** due to modular design
- **Better maintainability** with clear boundaries

## Archived Files

```
legacy-tests/
├── addition_task_webhook_integration_test.rs
├── circuit_breaker_test.rs
├── enhanced_rest_api_test.rs
├── graphql_playground_queries_test.rs
├── load_balancer_test.rs
├── output_config_validation_test.rs
├── output_delivery_integration_test.rs
├── output_destinations_graphql_test.rs
├── output_destinations_rest_api_test.rs
├── process_separation_integration_test.rs
├── rest_api_test.rs
├── task_execution_e2e_test.rs
├── validate_test.rs
└── common/
```

These files are preserved for historical reference but are not maintained or executed as part of the modern test suite.