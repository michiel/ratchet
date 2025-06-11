# Legacy Deprecation Plan

## Overview

This document outlines a systematic plan to deprecate and remove legacy constructs in the Ratchet codebase. During the repository synchronization implementation, several areas of technical debt and legacy code were identified that need modernization.

## Current State Analysis

### Legacy Systems Identified

#### 1. **Database Layer Duplication**
- **Legacy**: `ratchet-lib/src/database/` module with deprecated warnings
- **Modern**: `ratchet-storage` crate with `seaorm` implementation
- **Issue**: Dual database abstraction layers causing confusion and maintenance overhead
- **Impact**: HIGH - Core system functionality

#### 2. **Repository Factory Duplication**
```rust
// Legacy (deprecated)
ratchet_lib::database::repositories::RepositoryFactory

// Modern (SeaORM-based)  
ratchet_storage::seaorm::repositories::RepositoryFactory

// Abstract (interface-based)
ratchet_storage::RepositoryFactory
```

#### 3. **Server Implementation Duplication**
- **Legacy**: `serve_with_legacy_server()` in ratchet-cli
- **Modern**: `serve_with_ratchet_server()` using ratchet-server crate
- **Issue**: Fallback mechanism maintains two complete server implementations

#### 4. **Configuration Format Evolution**
- **Legacy**: `LibRatchetConfig` (ratchet-lib based)
- **Modern**: `RatchetConfig` (modular crate based)
- **Issue**: Config conversion layers and duplicate field definitions

#### 5. **Entity Model Duplication**
```rust
// Legacy (deprecated)
ratchet_lib::database::entities::{Task, Execution, Job, Schedule}

// Modern
ratchet_storage::entities::{task::Task, execution::Execution, ...}
```

## Deprecation Strategy

### Phase 1: Stabilize Modern Systems (Weeks 1-2)

#### 1.1 Complete ratchet-storage Migration
- [x] **Audit all ratchet-lib database usage** ✅ COMPLETED
  - ✅ Scanned codebase for `ratchet_lib::database` imports
  - ✅ Documented remaining dependencies on legacy database layer
  - ✅ Created compatibility matrix showing what needs migration
  - **Result**: Found 25 files requiring migration with clear mapping paths

- [x] **Implement missing ratchet-storage features** ✅ COMPLETED
  - ✅ Verified all legacy database functionality exists in ratchet-storage
  - ✅ Created comprehensive testing infrastructure in ratchet-storage:
    - TestDatabase utility with automatic cleanup and seeding
    - Builder patterns for all entity types (Task, Execution, Job, Schedule, DeliveryResult)
    - File fixtures for task directory and configuration testing
    - Mock implementations for repositories and services
    - Feature-gated testing modules with proper dependency management
  - ✅ Ensured feature parity for all database operations
  - **Result**: ratchet-storage now has equivalent testing capabilities to ratchet-lib

- [ ] **Create migration utilities** 🔄 IN PROGRESS
  - Build tools to migrate existing data from legacy schema to modern schema
  - Add database schema version detection and upgrade paths
  - Implement data validation after migration

#### 1.2 Server Architecture Consolidation
- [ ] **Improve ratchet-server reliability**
  - Fix remaining issues that cause fallback to legacy server
  - Add comprehensive error handling and logging
  - Implement graceful startup failure recovery

- [ ] **Add missing legacy server features to modern server**
  - Audit feature gaps between legacy and modern servers
  - Port critical functionality to ratchet-server implementation
  - Ensure API compatibility and behavioral consistency

### Phase 2: Create Migration Paths (Weeks 3-4)

#### 2.1 Database Layer Migration
- [ ] **Create ratchet-lib database wrapper**
  ```rust
  // Transitional approach - wrap modern implementation
  pub struct LegacyDatabaseAdapter {
      modern_impl: ratchet_storage::seaorm::repositories::RepositoryFactory,
  }
  ```
  - Implement legacy interface using modern backend
  - Maintain backward compatibility for existing consumers
  - Add deprecation warnings with migration guidance

- [ ] **Repository Pattern Unification**
  - Choose single repository abstraction (recommend: `ratchet_storage::repositories`)
  - Create adapters for SeaORM implementation to conform to abstract interface
  - Migrate all code to use unified repository pattern

#### 2.2 Configuration Modernization
- [ ] **Implement config auto-migration**
  - Detect legacy config formats and auto-upgrade
  - Preserve user settings during format transition
  - Add validation and error handling for config migration

- [ ] **Create config compatibility layer**
  - Support both legacy and modern config simultaneously
  - Gradually migrate components to use modern config
  - Add warnings for deprecated config options

### Phase 3: Mark Legacy Systems as Deprecated (Weeks 5-6)

#### 3.1 Add Deprecation Warnings
- [ ] **Database layer deprecations**
  ```rust
  #[deprecated(
      since = "0.4.0",
      note = "Use ratchet_storage crate instead. Will be removed in 0.5.0"
  )]
  pub mod database { ... }
  ```

- [ ] **Server implementation deprecations**
  - Mark `serve_with_legacy_server()` as deprecated
  - Add runtime warnings when fallback occurs
  - Provide clear migration guidance in error messages

- [ ] **Entity model deprecations**
  - Deprecate all legacy entity types
  - Add `#[deprecated]` attributes with migration paths
  - Update documentation to reference modern entities

#### 3.2 Update Documentation
- [ ] **Migration guides**
  - Create step-by-step migration documentation
  - Provide code examples for common migration patterns
  - Document breaking changes and workarounds

- [ ] **Architecture documentation updates**
  - Update ARCHITECTURE.md to reflect modern systems only
  - Remove references to deprecated components
  - Add decision records for deprecation choices

### Phase 4: Remove Legacy Code (Weeks 7-8)

#### 4.1 Legacy Database Removal
- [ ] **Remove ratchet-lib database module**
  - Delete `ratchet-lib/src/database/` directory
  - Remove database-related exports from ratchet-lib
  - Update all imports to use ratchet-storage

- [ ] **Clean up repository factories**
  - Remove `convert_to_legacy_repository_factory()` function
  - Simplify server startup to use single repository system
  - Remove compatibility adapters

#### 4.2 Server Implementation Cleanup
- [ ] **Remove legacy server**
  - Delete `serve_with_legacy_server()` function
  - Remove fallback mechanism from `serve_command_with_config()`
  - Simplify server startup flow

- [ ] **Configuration cleanup**
  - Remove `LibRatchetConfig` type
  - Delete config conversion utilities
  - Use only modern config throughout codebase

### Phase 5: Validation and Testing (Weeks 9-10)

#### 5.1 Comprehensive Testing
- [ ] **Migration testing**
  - Test upgrade paths from each supported legacy version
  - Verify data integrity after migration
  - Test error handling for corrupted legacy data

- [ ] **Regression testing**
  - Ensure all existing functionality works with modern systems
  - Test API compatibility and behavioral consistency
  - Verify performance improvements from legacy removal

#### 5.2 Documentation and Release
- [ ] **Update documentation**
  - Remove all references to deprecated systems
  - Update installation and configuration guides
  - Create changelog documenting breaking changes

- [ ] **Release planning**
  - Plan major version bump (0.4.0 → 0.5.0)
  - Communicate breaking changes to users
  - Provide migration timeline and support

## Implementation Timeline

```
Week 1-2:  Phase 1 - Stabilize Modern Systems
Week 3-4:  Phase 2 - Create Migration Paths  
Week 5-6:  Phase 3 - Mark Legacy as Deprecated
Week 7-8:  Phase 4 - Remove Legacy Code
Week 9-10: Phase 5 - Validation and Testing
```

## Risk Mitigation

### High Risk Areas
1. **Database Migration**: Data loss or corruption during schema migration
2. **API Compatibility**: Breaking changes affecting existing integrations
3. **Configuration**: Loss of user settings during config migration

### Mitigation Strategies
1. **Backup and Rollback**: Implement automatic backup before migrations
2. **Gradual Migration**: Support both systems simultaneously during transition
3. **Extensive Testing**: Comprehensive test coverage for migration paths
4. **User Communication**: Clear documentation and advance notice of changes

## Success Metrics

### Code Quality Improvements
- [ ] Reduce codebase size by ~15% (estimated 3000+ lines removed)
- [ ] Eliminate all deprecation warnings
- [ ] Achieve single-responsibility for each system component

### Performance Improvements
- [ ] Reduce server startup time (eliminate fallback overhead)
- [ ] Simplify database connection management
- [ ] Reduce memory footprint from duplicate systems

### Developer Experience
- [ ] Simplified architecture with clear separation of concerns
- [ ] Reduced cognitive load for new contributors
- [ ] Consistent patterns across all modules

## Notes

This deprecation plan addresses the technical debt identified during repository synchronization implementation. The dual database systems, server implementations, and configuration formats create maintenance overhead and confusion for developers.

The plan prioritizes user experience by maintaining backward compatibility during transition while providing clear migration paths to modern systems.

Implementation should be coordinated with regular releases to provide users adequate time to adapt to changes.