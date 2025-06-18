# Dependency Reduction Analysis Report

**Date:** 2025-06-18  
**Project:** Ratchet  
**Analysis Scope:** Workspace dependencies optimization without functionality changes

## Executive Summary

This analysis examined the Ratchet workspace's 26 crates and identified significant opportunities to reduce dependencies without changing functionality. The workspace currently has 105+ workspace-level dependencies and shows patterns of unused dependencies, version duplications, and potential consolidation opportunities.

**Key Findings:**
- **67 unused dependencies** identified across 22 crates using `cargo-machete`
- **Multiple version duplications** for base64, bytes, http, bitflags, and other core crates
- **Feature overlap** between HTTP-related crates that could be consolidated
- **Overly broad default features** in the main CLI crate leading to unnecessary dependencies

## Detailed Analysis

### 1. Unused Dependencies (High Impact)

cargo-machete identified 67 unused dependencies across the workspace. Key problem areas:

#### Main CLI Crate (`ratchet-cli`)
**Unused:** `axum`, `env_logger`, `futures`, `log`, `ratchet-caching`, `ratchet-cli-tools`, `ratchet-output`, `ratchet-plugins`, `ratchet-resilience`, `serde`, `thiserror`, `tokio-stream`, `tracing-appender`

**Root Cause:** The CLI uses feature flags but many dependencies are unconditionally imported in Cargo.toml while actually being feature-gated in the code.

#### Core Infrastructure Crates
- **ratchet-core:** `anyhow`, `log`, `serde_yaml`, `envy`, `humantime-serde` (5 unused)
- **ratchet-ipc:** `anyhow`, `bytes`, `futures`, `log`, `ratchet-core` (5 unused) 
- **ratchet-storage:** `anyhow`, `ratchet-caching`, `sqlx` (3 unused)

#### Server Components
- **ratchet-mcp:** `clap`, `humantime-serde`, `hyper`, `ratchet-js`, `tower`, `tracing-subscriber` (6 unused)
- **ratchet-server:** `async-graphql`, `async-graphql-axum`, `clap`, `ratchet-logging`, `serde_yaml`, `thiserror`, `tower`, `tower-http` (8 unused)

### 2. Version Duplications (Medium Impact)

Multiple versions of core dependencies are being pulled in:

#### Base64 (3 versions)
- `base64 v0.13.1` (via http-types/wiremock)
- `base64 v0.21.7` (via wiremock)  
- `base64 v0.22.1` (current workspace standard)

#### HTTP Stack Versions
- `http v0.2.12` and `http v1.3.1` 
- `hyper v0.14.32` and `hyper v1.6.0`
- `tower-http v0.5.2` and `tower-http v0.6.6`

#### System Libraries
- `bitflags v1.3.2` and `bitflags v2.9.1`
- `nix v0.28.0` and `nix v0.30.1`
- `rustix v0.38.44` and `rustix v1.0.7`

### 3. Functionality Duplication Opportunities

#### HTTP Client Consolidation
Currently have separate crates for HTTP concerns:
- `ratchet-http`: HTTP client functionality
- `ratchet-web`: Web middleware and utilities  
- `ratchet-rest-api`: REST API implementation
- `ratchet-mcp`: Contains its own HTTP server/client logic

**Opportunity:** Consolidate HTTP client logic into `ratchet-http` and reduce duplication in `ratchet-mcp`.

#### Error Handling Overlap
Many crates depend on both `anyhow` and `thiserror`:
- Could standardize on `thiserror` for library crates  
- Use `anyhow` only in application crates

#### Serialization Dependencies
Multiple crates import `serde`, `serde_json`, and `serde_yaml` but many show as unused:
- `ratchet-core`, `ratchet-plugins`, `ratchet-runtime` all have unused `serde`
- `ratchet-core`, `ratchet-server` have unused `serde_yaml`

### 4. Workspace Configuration Issues

#### Overly Permissive Default Features
The main CLI crate has `default = ["server", "database", "mcp-server", "plugins", "javascript", "output", "runtime", "http", "git"]` which forces inclusion of many dependencies even for simple CLI operations.

#### Missing Feature Gates
Some dependencies are imported unconditionally but only used in specific features:
- Database libraries when database features are disabled
- Server dependencies for CLI-only builds

## Recommendations

### Priority 1: Remove Unused Dependencies (Immediate Impact)

1. **Clean up main CLI crate:**
   ```toml
   # Remove unused direct dependencies
   # Move feature-specific deps behind feature gates
   axum = { workspace = true, optional = true }
   ratchet-caching = { path = "../ratchet-caching", optional = true }
   # ... etc for each unused dependency
   ```

2. **Fix feature gating in all crates:**
   - Move optional dependencies behind proper feature flags
   - Ensure feature dependencies are correctly specified

3. **Remove unused dependencies from Cargo.toml files** identified by cargo-machete

**Estimated reduction:** 67 dependencies removed from compilation

### Priority 2: Consolidate HTTP Infrastructure (Medium Impact)

1. **Merge HTTP client functionality:**
   - Move `ratchet-mcp` HTTP client logic to `ratchet-http`
   - Standardize on single HTTP client pattern across crates

2. **Unify web middleware:**
   - Evaluate if `ratchet-web` utilities can be moved to `ratchet-rest-api`
   - Reduce API surface duplication

**Estimated reduction:** 2-3 crates consolidated, ~15 dependencies removed

### Priority 3: Address Version Duplications (Medium Impact)

1. **Update dev dependencies:**
   - Upgrade wiremock to newer version that uses current base64
   - Remove older HTTP stack versions

2. **Standardize on current versions:**
   - Pin bitflags to v2.x across workspace
   - Update nix and rustix to latest stable versions

**Estimated reduction:** 8-10 duplicate dependency versions eliminated

### Priority 4: Optimize Default Features (Low Impact, High User Value)

1. **Create minimal CLI profile:**
   ```toml
   default = ["core"]
   minimal = ["core"]
   standard = ["database", "http", "git"]
   full = ["server", "database", "mcp-server", "plugins", "javascript", "output", "runtime", "http", "git"]
   ```

2. **Make server components truly optional:**
   - Ensure CLI works without server dependencies
   - Provide clear feature documentation

**Estimated impact:** Faster compilation for CLI-only builds, smaller binaries

## Implementation Plan

### Phase 1: Quick Wins (1-2 days)
- Remove unused dependencies identified by cargo-machete
- Fix obvious feature gating issues
- Update workspace dependency versions for duplicates

### Phase 2: Consolidation (3-5 days)  
- Merge HTTP client functionality
- Standardize error handling patterns
- Review and merge similar utility crates

### Phase 3: Optimization (2-3 days)
- Restructure default features
- Create optimized build profiles
- Update documentation for new feature flags

## Risk Assessment

**Low Risk:**
- Removing unused dependencies (validated by cargo-machete)
- Fixing version duplications

**Medium Risk:** 
- HTTP infrastructure consolidation (requires testing across all server components)
- Feature flag restructuring (may affect existing user builds)

**Mitigation:**
- Comprehensive testing of all feature combinations
- Gradual rollout with deprecation warnings
- Clear migration documentation

## Expected Benefits

1. **Faster Compilation:** Fewer dependencies = faster build times
2. **Smaller Binaries:** Optional features properly gated = smaller distribution size
3. **Cleaner Architecture:** Less duplication = easier maintenance
4. **Better User Experience:** Appropriate defaults for different use cases

## Conclusion

The Ratchet workspace has significant opportunities for dependency reduction without functionality loss. The analysis identified 67 unused dependencies and multiple consolidation opportunities. Implementing these recommendations would result in:

- **20-30% reduction** in total dependency count
- **Faster build times** especially for minimal CLI builds  
- **Cleaner architecture** with less duplication
- **Better user experience** with appropriate feature defaults

The recommended approach prioritizes low-risk, high-impact changes first, followed by more complex consolidation work.