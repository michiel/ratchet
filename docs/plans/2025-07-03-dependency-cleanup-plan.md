# Dependency and Code Cleanup Plan

**Date:** 2025-07-03  
**Priority:** High  
**Status:** Analysis Complete  

## Overview

This document outlines a comprehensive plan to reduce compilation time and binary size by removing unused dependencies, dead code, and optimizing the codebase. The analysis identified significant opportunities for cleanup across the Ratchet workspace.

## Current Analysis Results

### 1. Unused Dependencies (High Impact)

**Immediate Removals (cargo-machete findings):**

| Crate | Unused Dependencies | Impact |
|-------|-------------------|---------|
| `ratchet-cli` | `ratchet-caching`, `ratchet-cli-tools`, `ratchet-interfaces`, `ratchet-output`, `ratchet-plugins`, `ratchet-registry`, `ratchet-resilience` | High - Multiple large crates |
| `ratchet-js` | `ratchet-logging` | Medium - Logging integration (conditional) |
| `ratchet-http` | `axum`, `http`, `tokio`, `tower-http` | Medium - Web framework components |
| `ratchet-storage` | `sqlx` | Medium - Database abstraction |

**Workspace Dependencies Never Used:**
- `lazy_static` - Use `once_cell` instead
- `hostname` - Not used anywhere
- `unicode-width` - Not used anywhere
- `proptest` - Should be dev-dependency
- `criterion` - Should be dev-dependency

### 2. Duplicate Dependencies (Medium Impact)

**Version Conflicts:**
- `axum` v0.7.9 and v0.8.4 (both in dependency tree)
- `tower-http` v0.5.2 and v0.6.6
- `base64` v0.13.1, v0.21.7, and v0.22.1
- `bitflags` v1.3.2 and v2.9.1
- `bytes` v0.4.12 and v1.10.1
- `http` v0.2.12 and v1.3.1

### 3. Dead Code (Medium Impact)

**Large Unused Files:**
- `/ratchet-mcp/examples/ratchet_server_example.rs.disabled` (5,062 lines)
- `/ratchet-cli/src/main_new.rs` (alternative main file)

**Unused Struct Fields:**
- `ratchet-registry/src/loaders/filesystem.rs`: `base_path` field
- `ratchet-registry/src/loaders/git.rs`: `auth_manager`, `config` fields
- `ratchet-mcp/src/server/mod.rs`: `server_issued_sessions`, `message_history`

**Unused Functions/Methods:**
- 20+ unused imports across modules
- Dead code in handler functions
- Commented-out code blocks

### 4. Feature Flag Optimization (Low-Medium Impact)

**Over-featured Dependencies:**
- `async-graphql` - includes `graphiql` feature for development only
- `sqlx` - multiple database backends when only SQLite is used
- `tower-http` - many features enabled that may not be needed

## Implementation Plan

### Phase 1: Remove Unused Dependencies (Week 1)

**Priority 1: High Impact Removals**
1. Remove unused dependencies from `ratchet-cli/Cargo.toml`
2. Remove unused workspace dependencies (`lazy_static`, `hostname`, `unicode-width`)
3. Move dev-only dependencies to `[workspace.dev-dependencies]`
4. Review conditional dependencies (like `ratchet-logging` in `ratchet-js`)

**Commands to Execute:**
```bash
# Remove from ratchet-cli/Cargo.toml
ratchet-caching = { workspace = true, optional = true }  # Remove if truly unused
ratchet-cli-tools = { workspace = true, optional = true }  # Remove if truly unused
# ... (remove other unused optional dependencies)

# Remove from workspace Cargo.toml
lazy_static = "1.4"  # Remove - use once_cell
hostname = "0.4"     # Remove - not used
unicode-width = "0.2.1"  # Remove - not used

# Note: boa_runtime is REQUIRED for JavaScript execution - do NOT remove
```

### Phase 2: Resolve Version Conflicts (Week 2)

**Priority 2: Consolidate Versions**
1. Standardize on `axum` v0.8.4 across all crates
2. Standardize on `tower-http` v0.6.6
3. Consolidate `base64`, `bitflags`, `bytes`, `http` versions
4. Update dependent crates to use consistent versions

**Estimated Impact:** 15-20% reduction in compilation time

### Phase 3: Remove Dead Code (Week 3)

**Priority 3: Code Cleanup**
1. Delete disabled example file (saves 5,062 lines)
2. Remove unused imports and struct fields
3. Clean up commented code blocks
4. Remove unused functions and methods

**Commands to Execute:**
```bash
# Remove large disabled file
rm ratchet-mcp/examples/ratchet_server_example.rs.disabled

# Fix unused imports (example)
# Remove unused imports from axum-mcp-external/src/server/handler.rs
```

### Phase 4: Optimize Feature Flags (Week 4)

**Priority 4: Feature Optimization**
1. Review and minimize `async-graphql` features
2. Optimize `sqlx` to only include SQLite support
3. Review `tower-http` feature usage
4. Add conditional compilation for development-only features

**Example Changes:**
```toml
# More targeted feature selection
async-graphql = { version = "7.0", features = ["uuid", "chrono"], default-features = false }
sqlx = { version = "0.8", features = ["runtime-tokio-rustls", "sqlite"], default-features = false }
```

## Expected Benefits

### Compilation Time Improvements
- **Phase 1:** 25-35% reduction (removing large unused dependencies)
- **Phase 2:** 10-15% additional reduction (version consolidation)
- **Phase 3:** 5-10% additional reduction (dead code removal)
- **Phase 4:** 5-10% additional reduction (feature optimization)

**Total Expected:** 45-70% compilation time reduction

### Binary Size Improvements
- **JavaScript Engine Removal:** ~129MB reduction
- **Dead Code Removal:** ~10-20MB reduction
- **Feature Optimization:** ~5-15MB reduction

**Total Expected:** 140-165MB binary size reduction

### Code Maintainability
- Reduced cognitive load from unused code
- Cleaner dependency graph
- Faster CI/CD pipelines
- Improved development experience

## Risk Assessment

### Low Risk Changes
- Removing unused workspace dependencies
- Removing dead code and unused imports
- Deleting disabled example files

### Medium Risk Changes
- Version consolidation (requires testing)
- Feature flag optimization (may break functionality)

### High Risk Changes
- Major dependency removal from core crates that affect functionality
- Changing feature flags that disable core capabilities

## Implementation Steps

### Step 1: Backup and Preparation
1. Create feature branch: `cleanup/dependency-optimization`
2. Run full test suite to establish baseline
3. Document current binary sizes and compilation times

### Step 2: Execute Phase 1 (High Impact)
1. Remove unused dependencies from Cargo.toml files
2. Clean up workspace dependencies
3. Test compilation and basic functionality

### Step 3: Execute Phases 2-4 (Incremental)
1. Implement one phase at a time
2. Test after each phase
3. Measure performance improvements
4. Document changes and benefits

### Step 4: Validation and Documentation
1. Run full test suite
2. Measure final performance improvements
3. Update documentation
4. Create summary report

## Success Metrics

- [ ] **Compilation Time:** Achieve 45%+ reduction
- [ ] **Binary Size:** Achieve 140MB+ reduction
- [ ] **Test Coverage:** Maintain 100% test pass rate
- [ ] **Functionality:** No regressions in core features
- [ ] **Documentation:** Updated dependency documentation

## Notes

- Changes should be implemented incrementally with testing at each step
- Some dependencies may be transitively required even if not directly used
- Feature flag changes should be carefully tested to avoid breaking functionality
- Consider creating a `[workspace.dev-dependencies]` section for development-only dependencies