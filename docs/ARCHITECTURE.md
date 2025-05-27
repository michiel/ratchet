# Ratchet Architecture Guide

This document outlines the architecture, design principles, and conventions used in the Ratchet codebase.

## Table of Contents

- [Overview](#overview)
- [Code Layout](#code-layout)
- [Module Structure](#module-structure)
- [Conventions](#conventions)
- [Error Handling](#error-handling)
- [Type Safety](#type-safety)
- [Testing Strategy](#testing-strategy)
- [Development Guidelines](#development-guidelines)

## Overview

Ratchet is a JavaScript task execution framework written in Rust, designed with modularity, type safety, and maintainability as core principles. The architecture follows a layered approach with clear separation of concerns.

### Core Components

- **Task Management**: Loading, validation, and execution of JavaScript tasks
- **JavaScript Engine**: Secure JavaScript execution environment using Boa
- **HTTP Client**: Type-safe HTTP request handling with mock support
- **Validation**: JSON schema validation for inputs and outputs
- **Recording**: Session recording and replay functionality
- **CLI Interface**: Command-line interface for task operations

## Code Layout

### Workspace Structure

```
ratchet/
├── ratchet-lib/          # Core library functionality
│   └── src/
├── ratchet-cli/          # Command-line interface
│   └── src/
├── sample/               # Example tasks and test data
├── docs/                 # Documentation
└── target/               # Build artifacts
```

### Library Module Organization

The `ratchet-lib` crate is organized into focused, single-responsibility modules:

```
ratchet-lib/src/
├── lib.rs                # Public API and module exports (30 lines)
├── errors.rs             # Centralized error type definitions (65 lines)
├── types.rs              # Type-safe enums and conversions (396 lines)
├── js_executor.rs        # JavaScript execution engine (588 lines)
├── task.rs               # Task loading and management (713 lines)
├── test.rs               # Test execution framework (449 lines)
├── generate.rs           # Task template generation (298 lines)
├── js_task.rs            # JavaScript task wrapper (107 lines)
├── validation/           # JSON schema validation
│   ├── mod.rs            # Module exports (2 lines)
│   └── schema.rs         # Validation logic (28 lines)
├── recording/            # Session recording functionality
│   ├── mod.rs            # Module exports (5 lines)
│   └── session.rs        # Recording implementation (216 lines)
└── http/                 # HTTP client functionality
    ├── mod.rs            # Module exports (9 lines)
    ├── manager.rs        # HTTP client implementation (307 lines)
    ├── errors.rs         # HTTP-specific errors (28 lines)
    ├── fetch.rs          # JavaScript fetch integration (120 lines)
    └── tests.rs          # HTTP testing suite (272 lines)
```

### Design Principles

1. **Single Responsibility**: Each module has one clear purpose
2. **Minimal Dependencies**: Modules depend only on what they need
3. **Clear Interfaces**: Public APIs are well-defined and documented
4. **Type Safety**: Strong typing throughout with minimal `unwrap()`
5. **Error Handling**: Comprehensive error types with context
6. **Testability**: All modules are thoroughly tested

## Module Structure

### Core Modules

#### `lib.rs` - Public API
- **Purpose**: Module exports and public API surface
- **Size**: 30 lines (97% reduction from original 1063 lines)
- **Contents**: Module declarations and re-exports for convenience
- **Dependencies**: All other modules

#### `errors.rs` - Error Types
- **Purpose**: Centralized error type definitions
- **Contents**: `JsErrorType`, `JsExecutionError` with comprehensive error variants
- **Design**: Hierarchical error types with rich context information

#### `types.rs` - Type Safety
- **Purpose**: Type-safe enums replacing string-based types
- **Contents**: `HttpMethod`, `LogLevel`, `TaskStatus` with conversions
- **Features**: Serialization, parsing, validation, and error handling

#### `js_executor.rs` - JavaScript Engine
- **Purpose**: JavaScript task execution and environment management
- **Contents**: Boa engine integration, error type registration, HTTP integration
- **Key Functions**: `execute_task()`, `call_js_function()`, error handling

#### `task.rs` - Task Management
- **Purpose**: Task loading, validation, and lifecycle management
- **Contents**: Task struct, file/ZIP loading, content caching, validation
- **Features**: Lazy loading, LRU caching, ZIP support

### Supporting Modules

#### `validation/` - Schema Validation
- **Purpose**: JSON schema validation for task inputs/outputs
- **Structure**: 
  - `schema.rs`: Core validation logic using jsonschema crate
  - `mod.rs`: Public API exports
- **Integration**: Used by js_executor for input/output validation

#### `recording/` - Session Recording
- **Purpose**: HTTP request recording and session management
- **Structure**:
  - `session.rs`: Recording state management and HAR file generation
  - `mod.rs`: Public API exports
- **Features**: HAR format output, thread-safe recording state

#### `http/` - HTTP Client
- **Purpose**: HTTP request handling with mock support
- **Structure**:
  - `manager.rs`: Main HTTP client implementation
  - `errors.rs`: HTTP-specific error types
  - `fetch.js`: JavaScript fetch API integration
  - `tests.rs`: Comprehensive test suite
  - `mod.rs`: Module exports and public API

## Conventions

### Naming Conventions

#### Modules
- **snake_case**: All module names use snake_case (e.g., `js_executor`, `http_manager`)
- **Descriptive**: Names clearly indicate module purpose
- **Consistent**: Related functionality grouped under common prefixes

#### Types
- **PascalCase**: All type names use PascalCase (e.g., `HttpMethod`, `TaskStatus`)
- **Descriptive**: Names indicate the type's purpose and domain
- **Suffixed**: Error types end with `Error` (e.g., `JsExecutionError`)

#### Functions
- **snake_case**: All function names use snake_case
- **Verb-based**: Functions start with verbs (e.g., `execute_task`, `validate_json`)
- **Clear intent**: Names indicate what the function does

#### Constants
- **SCREAMING_SNAKE_CASE**: All constants use SCREAMING_SNAKE_CASE
- **Descriptive**: Names clearly indicate the constant's purpose
- **Grouped**: Related constants are grouped together

### Code Organization

#### File Structure
```rust
// 1. Imports - organized by scope
use std::collections::HashMap;     // Standard library
use serde::{Deserialize, Serialize}; // External crates  
use crate::errors::HttpError;      // Internal modules

// 2. Types - public then private
pub struct PublicType { }
struct PrivateType { }

// 3. Constants
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

// 4. Implementations
impl PublicType {
    pub fn new() -> Self { }       // Constructors first
    pub fn public_method(&self) { } // Public methods
    fn private_method(&self) { }   // Private methods
}

// 5. Functions - public then private
pub fn public_function() { }
fn private_function() { }

// 6. Tests
#[cfg(test)]
mod tests { }
```

#### Import Organization
1. **Standard library**: `std::*` imports
2. **External crates**: Third-party dependencies
3. **Internal modules**: `crate::*` imports
4. **Blank lines**: Separate each group

#### Documentation
- **Module docs**: Every public module has comprehensive documentation
- **Function docs**: All public functions have doc comments
- **Example usage**: Complex APIs include usage examples
- **Error documentation**: Error conditions are documented

### Error Handling Patterns

#### Result Types
```rust
// Always use Result for fallible operations
pub fn execute_task(task: &Task) -> Result<JsonValue, JsExecutionError> {
    // Implementation
}

// Use specific error types, not generic Error
pub fn parse_schema(path: &Path) -> Result<JsonValue, JsExecutionError> {
    // Implementation
}
```

#### Error Propagation
```rust
// Use ? operator for error propagation
pub fn complex_operation() -> Result<(), MyError> {
    let data = load_data()?;          // Propagate LoadError
    let processed = process(data)?;    // Propagate ProcessError
    save_result(processed)?;          // Propagate SaveError
    Ok(())
}

// Add context when helpful
pub fn load_task(path: &Path) -> Result<Task, TaskError> {
    Task::from_fs(path)
        .with_context(|| format!("Failed to load task from: {}", path.display()))
}
```

## Error Handling

### Error Type Hierarchy

```rust
// Top-level error categories
pub enum JsExecutionError {
    FileReadError(#[from] std::io::Error),
    CompileError(String),
    ExecutionError(String),
    TypedJsError(#[from] JsErrorType),
    SchemaValidationError(String),
    // ...
}

// Domain-specific JavaScript errors
pub enum JsErrorType {
    AuthenticationError(String),
    AuthorizationError(String),
    NetworkError(String),
    HttpError { status: u16, message: String },
    // ...
}
```

### Error Design Principles

#### 1. **Hierarchical Structure**
- **Category errors**: Broad error categories (e.g., `JsExecutionError`)
- **Specific errors**: Detailed error types (e.g., `AuthenticationError`)
- **Context preservation**: Errors maintain context through the call stack

#### 2. **Rich Error Information**
```rust
#[derive(Error, Debug)]
pub enum HttpError {
    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),

    #[error("Invalid HTTP method: {0}")]
    InvalidMethod(#[from] HttpMethodError),

    #[error("HTTP error {status}: {message}")]
    HttpStatusError { status: u16, message: String },
}
```

#### 3. **Error Conversion**
- **Automatic conversion**: Use `#[from]` for automatic conversions
- **Context addition**: Add context when converting between error types
- **Preservation**: Maintain original error information

#### 4. **User-Friendly Messages**
```rust
#[error("Invalid HTTP method: '{0}'. Supported methods are: GET, POST, PUT, DELETE, PATCH, HEAD, OPTIONS")]
InvalidMethod(String),

#[error("Invalid log level: '{0}'. Supported levels are: trace, debug, info, warn, error")]
InvalidLevel(String),
```

### Error Handling Best Practices

#### 1. **Fail Fast**
- Validate inputs early and return errors immediately
- Use type system to prevent errors at compile time
- Prefer `Result` over panics for recoverable errors

#### 2. **Error Context**
```rust
// Good: Provides context about what failed
fn load_task_file(path: &Path) -> Result<String, TaskError> {
    std::fs::read_to_string(path)
        .map_err(|e| TaskError::FileReadError {
            path: path.to_path_buf(),
            source: e,
        })
}

// Better: Use with_context for dynamic messages
fn process_task(name: &str) -> Result<Task, TaskError> {
    load_task_file(&format!("{}.json", name))
        .with_context(|| format!("Failed to process task: {}", name))
}
```

#### 3. **Error Recovery**
```rust
// Provide fallback mechanisms where appropriate
pub fn get_method_or_default(params: &JsonValue) -> HttpMethod {
    params.get("method")
        .and_then(|m| m.as_str())
        .and_then(|s| s.parse().ok())
        .unwrap_or(HttpMethod::Get)  // Safe default
}
```

## Type Safety

### Strongly Typed APIs

#### Replace String Types
```rust
// Before: Error-prone string handling
fn add_mock(method: &str, url: &str, response: JsonValue) {
    // "GET", "get", "Get" all different - runtime errors
}

// After: Compile-time safety
fn add_mock(method: HttpMethod, url: &str, response: JsonValue) {
    // Only valid HttpMethod values accepted
}
```

#### Enum Design
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum HttpMethod {
    Get, Post, Put, Delete, Patch, Head, Options
}

impl HttpMethod {
    pub fn as_str(&self) -> &'static str { /* ... */ }
    pub fn all() -> &'static [HttpMethod] { /* ... */ }
}

impl FromStr for HttpMethod {
    type Err = HttpMethodError;
    fn from_str(s: &str) -> Result<Self, Self::Err> { /* ... */ }
}
```

### Validation and Conversion

#### Parse, Don't Validate
```rust
// Good: Parse into validated type
pub fn parse_log_level(s: &str) -> Result<LogLevel, LogLevelError> {
    match s.to_lowercase().as_str() {
        "debug" => Ok(LogLevel::Debug),
        "info" => Ok(LogLevel::Info),
        // ...
        _ => Err(LogLevelError::InvalidLevel(s.to_string())),
    }
}

// Use the parsed type throughout the system
fn configure_logging(level: LogLevel) {
    // level is guaranteed to be valid
}
```

## Testing Strategy

### Test Organization

#### Module Tests
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_basic_functionality() { }
    
    #[tokio::test]
    async fn test_async_functionality() { }
}
```

#### Integration Tests
- **Location**: `tests/` directory in each crate
- **Purpose**: Test public APIs and cross-module interactions
- **Scope**: End-to-end functionality testing

#### Test Categories

1. **Unit Tests**: Test individual functions and methods
2. **Integration Tests**: Test module interactions
3. **Property Tests**: Test invariants and edge cases
4. **Performance Tests**: Benchmark critical paths

### Test Patterns

#### Arrange, Act, Assert
```rust
#[test]
fn test_http_method_parsing() {
    // Arrange
    let input = "POST";
    
    // Act
    let result = HttpMethod::from_str(input);
    
    // Assert
    assert_eq!(result.unwrap(), HttpMethod::Post);
}
```

#### Error Testing
```rust
#[test]
fn test_invalid_method_error() {
    let result = HttpMethod::from_str("INVALID");
    
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(error.to_string().contains("INVALID"));
    assert!(error.to_string().contains("GET, POST, PUT"));
}
```

## Development Guidelines

### Code Quality

#### 1. **Clippy Compliance**
- Run `cargo clippy` regularly and address warnings
- Use `#[allow(clippy::lint_name)]` sparingly and with justification
- Follow Clippy suggestions for idiomatic Rust

#### 2. **Formatting**
- Use `cargo fmt` for consistent code formatting
- Configure editor to format on save
- Follow Rust standard formatting conventions

#### 3. **Documentation**
```rust
/// Execute a JavaScript task with the given input data.
/// 
/// This function loads the task content, validates input against the schema,
/// executes the JavaScript code in a secure environment, and validates the output.
/// 
/// # Arguments
/// 
/// * `task` - The task to execute (will be modified to load content)
/// * `input_data` - Input data that must match the task's input schema
/// * `http_manager` - HTTP client for fetch API calls
/// 
/// # Returns
/// 
/// Returns the task output as JSON if successful, or a `JsExecutionError` if:
/// - The task content cannot be loaded
/// - Input validation fails
/// - JavaScript execution fails
/// - Output validation fails
/// 
/// # Example
/// 
/// ```rust
/// use ratchet_lib::{Task, HttpManager, execute_task};
/// use serde_json::json;
/// 
/// let mut task = Task::from_fs("path/to/task")?;
/// let input = json!({"num1": 5, "num2": 10});
/// let http_manager = HttpManager::new();
/// 
/// let result = execute_task(&mut task, input, &http_manager).await?;
/// println!("Result: {}", result);
/// ```
pub async fn execute_task(
    task: &mut Task,
    input_data: JsonValue,
    http_manager: &HttpManager,
) -> Result<JsonValue, JsExecutionError> {
    // Implementation
}
```

### Performance Considerations

#### 1. **Async/Await Usage**
- Use async functions for I/O operations
- Avoid blocking operations in async contexts
- Use `tokio::spawn` for independent concurrent tasks

#### 2. **Memory Management**
- Use `Arc` for shared ownership of immutable data
- Use `Rc` for single-threaded shared ownership
- Implement caching for expensive computations

#### 3. **Error Handling Performance**
- Use `Result` instead of exceptions for control flow
- Avoid string allocations in hot paths
- Use static strings for error messages when possible

### Security Guidelines

#### 1. **JavaScript Execution**
- Validate all inputs before JavaScript execution
- Limit resource usage in JavaScript environment
- Sanitize outputs from JavaScript execution

#### 2. **HTTP Requests**
- Validate URLs before making requests
- Implement request timeouts
- Use type-safe HTTP methods and headers

#### 3. **File Operations**
- Validate file paths to prevent directory traversal
- Use safe file operations with proper error handling
- Implement size limits for file operations

---

This architecture document serves as a living guide for maintaining and extending the Ratchet codebase. It should be updated as the architecture evolves and new patterns emerge.