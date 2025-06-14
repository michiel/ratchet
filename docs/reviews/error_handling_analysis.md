# Error Handling and Propagation Analysis

**Date**: 2024-12-21  
**Reviewer**: Claude Code  
**Scope**: Cross-API error handling consistency, sanitization, and reuse patterns

## Executive Summary

The Ratchet codebase demonstrates a sophisticated but inconsistent approach to error handling across its three main API surfaces (GraphQL, REST, MCP). While there are strong foundations in place, particularly around error sanitization and unified error types, there are significant opportunities for consolidation and improved consistency.

### Key Findings

✅ **Strengths:**
- Comprehensive error sanitization system in `ratchet-core`
- Unified API error types in `ratchet-api-types`
- Good HTTP status code mapping
- Strong error categorization and user-friendly messaging

❌ **Areas for Improvement:**
- Inconsistent adoption of unified error types across APIs
- Duplicated error conversion logic
- Inconsistent sanitization enforcement
- Limited cross-API error handling middleware reuse

## Detailed Analysis

### 1. Error Type Architecture

#### Core Error Types

The codebase uses a hierarchical error system:

```rust
// ratchet-core/src/error.rs
pub enum RatchetError {
    Task(TaskError),
    Execution(ExecutionError),
    Storage(StorageError),
    Config(ConfigError),
    Validation(ValidationError),
    // ... others
}
```

**Analysis:**
- ✅ Well-structured hierarchy with domain-specific error types
- ✅ Consistent error codes and HTTP status mapping
- ✅ Retryability logic built into error types
- ❌ Not consistently used across all API layers

#### Unified API Error Type

```rust
// ratchet-api-types/src/errors.rs
pub struct ApiError {
    pub code: String,
    pub message: String,
    pub request_id: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub path: Option<String>,
    pub details: Option<serde_json::Value>,
    pub suggestions: Option<Vec<String>>,
}
```

**Analysis:**
- ✅ Excellent unified structure for cross-API consistency
- ✅ Rich metadata including suggestions and context
- ✅ Built-in timestamp and tracing support
- ❌ Not consistently adopted across all APIs

### 2. Error Sanitization

#### Core Sanitization System

The `ratchet-core/src/validation/error_sanitization.rs` module provides comprehensive error sanitization:

```rust
pub struct ErrorSanitizer {
    config: ErrorSanitizationConfig,
    sensitive_patterns: Vec<Regex>,
    path_patterns: Vec<Regex>,
}
```

**Capabilities:**
- ✅ Removes sensitive information (connection strings, tokens, file paths)
- ✅ Categorizes errors by type (database, auth, validation, etc.)
- ✅ Configurable sanitization rules
- ✅ Custom error mappings support
- ✅ Length limits and safe fallbacks

**Coverage Analysis:**
- ✅ Database connection strings: `postgresql://user:pass@host/db` → `[REDACTED]`
- ✅ File paths: `/home/user/secret/file.txt` → `[PATH]`
- ✅ JWT tokens and API keys
- ✅ Stack traces and debug information
- ✅ Environment variables

### 3. API-Specific Error Handling

#### GraphQL API (`ratchet-graphql-api`)

```rust
// Limited error types, delegates to unified system
pub enum GraphQLError {
    Repository(DatabaseError),
    Registry(RegistryError),
}

impl From<GraphQLError> for ApiError {
    fn from(error: GraphQLError) -> Self {
        match error {
            GraphQLError::Repository(e) => ApiError::internal_error(format!("Database error: {}", e)),
            GraphQLError::Registry(e) => ApiError::internal_error(format!("Registry error: {}", e)),
        }
    }
}
```

**Analysis:**
- ✅ Uses unified `ApiError` type
- ✅ GraphQL extensions support for error metadata
- ❌ Limited error categorization
- ❌ **No evidence of sanitization usage**
- ❌ Simple error mapping loses context

#### REST API (`ratchet-rest-api`)

```rust
pub enum RestError {
    NotFound(String),
    BadRequest(String),
    InternalError(String),
    // ... others
    Database(DatabaseError),
    Web(WebError),
    Validation { message: String },
}

impl RestError {
    pub fn to_unified_error(&self) -> ApiError {
        match self {
            RestError::NotFound(msg) => ApiError::not_found("Resource", msg),
            // ... detailed mappings
        }
    }
}
```

**Analysis:**
- ✅ Comprehensive HTTP status code mapping
- ✅ Converts to unified `ApiError`
- ✅ Good error categorization
- ❌ **No evidence of sanitization usage**
- ❌ Some duplication with `WebError`

#### MCP API (`ratchet-mcp`)

```rust
pub enum McpError {
    Transport { message: String },
    Protocol { message: String },
    ToolNotFound { tool_name: String },
    AuthenticationFailed { reason: String },
    // ... many others
}

impl McpError {
    pub fn is_retryable(&self) -> bool { /* ... */ }
    pub fn retry_delay(&self) -> Option<Duration> { /* ... */ }
}
```

**Analysis:**
- ✅ Rich error types specific to MCP protocol
- ✅ Built-in retry logic
- ✅ Good error categorization
- ❌ **No conversion to unified `ApiError`**
- ❌ **No evidence of sanitization usage**
- ❌ Completely separate error hierarchy

### 4. Error Propagation Patterns

#### Current Flow

```
Core Domain Error → API-Specific Error → HTTP/GraphQL/MCP Response
```

#### Issues Identified

1. **Inconsistent Sanitization**: Only the core layer has sanitization, but it's not used by API layers
2. **Multiple Conversion Paths**: Each API has its own conversion logic
3. **Loss of Error Context**: Multiple conversion steps lose important context
4. **No Shared Middleware**: Each API implements its own error handling

### 5. Storage Layer Error Handling

```rust
// ratchet-storage/src/error.rs
impl StorageError {
    pub fn user_message(&self) -> &'static str {
        match self {
            StorageError::ConnectionFailed(_) => "Database connection unavailable",
            StorageError::NotFound => "Requested item not found",
            // ... safe user messages
        }
    }
}
```

**Analysis:**
- ✅ Good user-friendly message mapping
- ✅ Error code categorization
- ✅ Retryability logic
- ❌ **Manual sanitization instead of using core sanitizer**

### 6. Cross-API Consistency Issues

| Feature | GraphQL | REST | MCP | Status |
|---------|---------|------|-----|--------|
| Unified Error Type | ✅ | ✅ | ❌ | Inconsistent |
| Error Sanitization | ❌ | ❌ | ❌ | Not Implemented |
| Error Codes | ✅ | ✅ | ✅ | Consistent |
| HTTP Status Mapping | ✅ | ✅ | N/A | Good |
| Retry Logic | ❌ | ❌ | ✅ | Inconsistent |
| Request Tracing | ✅ | ✅ | ❌ | Inconsistent |

## Critical Security Issues

### 1. Sanitization Not Enforced

**Issue**: The sophisticated error sanitization system exists but is not used by any API layer.

**Risk**: Internal information leakage through error messages

**Evidence**: No imports of `ErrorSanitizer` found in API modules

### 2. Inconsistent Error Boundaries

**Issue**: Different APIs handle errors at different layers, leading to inconsistent information exposure.

**Example**:
```rust
// In GraphQL - direct error forwarding
GraphQLError::Repository(e) => ApiError::internal_error(format!("Database error: {}", e))

// In Storage - manual sanitization
StorageError::ConnectionFailed(_) => "Database connection unavailable"
```

### 3. Missing Error Context Validation

**Issue**: No validation that error messages are safe before crossing API boundaries.

## Recommendations

### Priority 1: Critical Security Fixes

1. **Enforce Error Sanitization**
   ```rust
   // Add to all API error conversion points
   use ratchet_core::validation::error_sanitization::ErrorSanitizer;
   
   impl From<DatabaseError> for ApiError {
       fn from(error: DatabaseError) -> Self {
           let sanitizer = ErrorSanitizer::default();
           let sanitized = sanitizer.sanitize_error(&error);
           ApiError::new(sanitized.error_code.unwrap_or("DATABASE_ERROR"), sanitized.message)
       }
   }
   ```

2. **Add Sanitization Middleware**
   ```rust
   // Shared middleware for all APIs
   pub async fn error_sanitization_middleware<B>(
       request: Request<B>,
       next: Next<B>,
   ) -> Response {
       let response = next.run(request).await;
       // Sanitize any error responses before returning
       sanitize_error_response(response)
   }
   ```

### Priority 2: Consolidation Improvements

1. **Unified Error Conversion Trait**
   ```rust
   pub trait ToSanitizedApiError {
       fn to_sanitized_api_error(&self) -> ApiError;
   }
   
   impl<E: std::error::Error> ToSanitizedApiError for E {
       fn to_sanitized_api_error(&self) -> ApiError {
           let sanitizer = ErrorSanitizer::default();
           let sanitized = sanitizer.sanitize_error(self);
           // Convert to ApiError with proper categorization
       }
   }
   ```

2. **Shared Error Middleware Crate**
   ```rust
   // New crate: ratchet-error-middleware
   pub mod graphql;
   pub mod rest;
   pub mod mcp;
   pub mod common;
   ```

3. **MCP Integration with Unified Errors**
   ```rust
   impl From<McpError> for ApiError {
       fn from(error: McpError) -> Self {
           // Convert MCP errors to unified format
       }
   }
   ```

### Priority 3: Enhanced Features

1. **Structured Error Logging**
   ```rust
   pub struct ErrorLogger {
       sanitizer: ErrorSanitizer,
   }
   
   impl ErrorLogger {
       pub fn log_api_error(&self, error: &dyn std::error::Error, context: ErrorContext) {
           // Log full error internally, sanitized error to API response
       }
   }
   ```

2. **Error Metrics and Monitoring**
   ```rust
   pub struct ErrorMetrics {
       error_counts: HashMap<String, u64>,
       sanitization_triggers: u64,
   }
   ```

3. **Configuration-Driven Sanitization**
   ```rust
   // Allow runtime configuration of sanitization rules
   pub struct SanitizationConfig {
       rules: Vec<SanitizationRule>,
       enforcement_level: EnforcementLevel,
   }
   ```

## Implementation Plan

### Phase 1: Security Critical (1-2 weeks)
- [ ] Add error sanitization to all API conversion points
- [ ] Create sanitization middleware for REST API
- [ ] Add GraphQL error sanitization extensions
- [ ] Add MCP error sanitization in message serialization

### Phase 2: Consistency (2-3 weeks)
- [ ] Create unified error conversion trait
- [ ] Migrate MCP to use unified `ApiError` where appropriate
- [ ] Add shared error middleware crate
- [ ] Standardize error logging across APIs

### Phase 3: Enhancement (1-2 weeks)
- [ ] Add error metrics and monitoring
- [ ] Implement configuration-driven sanitization
- [ ] Add comprehensive error handling tests
- [ ] Document error handling best practices

## Testing Strategy

### 1. Sanitization Tests
```rust
#[test]
fn test_api_error_sanitization() {
    let db_error = DatabaseError::ConnectionFailed("postgresql://user:pass@host/db".to_string());
    let api_error = db_error.to_sanitized_api_error();
    assert!(!api_error.message.contains("postgresql://"));
    assert!(!api_error.message.contains("user:pass"));
}
```

### 2. Cross-API Consistency Tests
```rust
#[test]
fn test_error_consistency_across_apis() {
    let core_error = RatchetError::Task(TaskError::NotFound("task-123".to_string()));
    
    let graphql_error = ApiError::from(core_error.clone());
    let rest_error = RestError::from(core_error.clone()).to_unified_error();
    let mcp_error = McpError::from(core_error).to_unified_error();
    
    assert_eq!(graphql_error.code, rest_error.code);
    assert_eq!(rest_error.code, mcp_error.code);
}
```

### 3. Security Tests
```rust
#[test]
fn test_no_sensitive_info_in_api_responses() {
    // Test that no sensitive patterns appear in any API error response
}
```

## Conclusion

The Ratchet codebase has excellent foundations for error handling with sophisticated sanitization and unified error types. However, these systems are not consistently applied across all API surfaces, creating security risks and inconsistent user experiences.

The recommended improvements focus on:
1. **Security**: Enforcing sanitization at all API boundaries
2. **Consistency**: Using unified error types and handling patterns
3. **Maintainability**: Consolidating error handling logic into shared modules

Priority should be given to the security critical Phase 1 improvements, as the current lack of sanitization enforcement poses a real risk of information leakage.

## Implementation Status Update

**Date**: 2025-06-14  
**Status**: Analysis Complete ✅ / Implementation Partially Complete ⚠️  
**Priority**: Critical Security Issue

### Completed Work
- ✅ Comprehensive error handling analysis across all APIs
- ✅ Security vulnerability identification 
- ✅ Designed shared error middleware architecture in `ratchet-error-middleware` crate
- ✅ Created comprehensive test plan for cross-API consistency
- ✅ Documented 4 key recommendations for implementation

### Partial Implementation  
- ⚠️ Created `ratchet-error-middleware` crate with full error handling utilities
- ⚠️ Implementation disabled due to axum 0.6 compatibility issues
- ⚠️ HTTP crate version conflicts between axum 0.6 and workspace dependencies
- ⚠️ Complex type system incompatibilities with middleware patterns

### Technical Issues Encountered
1. **Axum 0.6 Compatibility**: The workspace uses axum 0.6.20 which has different middleware patterns than axum 0.7+
2. **HTTP Crate Conflicts**: Multiple versions of the `http` crate causing type mismatches
3. **Body Handling**: Different body extraction patterns between axum versions
4. **Async GraphQL Compatibility**: GraphQL error extension APIs differ between async-graphql versions

### Next Steps Required
1. **Resolve axum compatibility**: Either upgrade entire workspace to axum 0.7+ or implement simplified error handling using axum 0.6 patterns
2. **Implement security fixes**: Apply error sanitization recommendations from this analysis using current API patterns  
3. **Cross-API standardization**: Ensure all APIs use consistent error formats with current codebase constraints
4. **Comprehensive testing**: Validate error handling across all API boundaries

### Immediate Recommendations
Given the critical security issues identified:

1. **Quick Security Fix**: Implement error sanitization directly in each API crate without shared middleware
2. **Gradual Migration**: Plan workspace upgrade to newer axum version for long-term middleware consolidation
3. **Documentation**: Update each API's error handling documentation to reflect current patterns

This analysis successfully identified critical security vulnerabilities and provides a clear roadmap for resolution, even though the full middleware implementation requires further dependency resolution.