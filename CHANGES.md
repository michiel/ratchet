# Changes

## v0.4.12 (TBD) - Full Task Database Storage Implementation

This release implements comprehensive task storage in the database with bidirectional repository synchronization. Tasks are now pulled from configured repositories and changes can be pushed back to their original sources.

### 🔄 **Breaking Changes**

#### Database Schema Changes
- **tasks table**: Added required `repository_id` column - all existing tasks will be assigned to a default filesystem repository during migration
- **tasks table**: Added `repository_path`, `sync_status`, `needs_push`, `source_code`, `source_type`, `checksum` columns
- **New tables**: `task_repositories` and `task_versions` for repository management and version tracking

#### API Changes
- **Task creation**: All new tasks must be assigned to a repository (automatically uses default if not specified)
- **Task responses**: Now include full repository information and source code
- **New endpoints**: Complete repository management API (`/api/v1/repositories/*`)

### ✨ **New Features**

#### Repository Management
- Full CRUD operations for task repositories via REST, GraphQL, and MCP APIs
- Support for filesystem, Git, and HTTP repository types
- Default repository assignment for new tasks
- Bidirectional sync: pull from repositories and push changes back

#### Enhanced Task Storage
- Complete task source code stored in database alongside metadata
- Task versioning and change tracking
- Repository-aware task assignment and management
- Conflict resolution for sync operations

#### API Enhancements
- **REST API**: New repository management endpoints
- **GraphQL**: Repository queries and mutations
- **MCP Protocol**: Repository operations support
- All APIs support full task CRUD with source code

### 🔧 **Migration Notes**

#### Automatic Migration
- Existing file-based tasks will be imported into database storage
- A default filesystem repository will be created automatically
- All existing tasks will be assigned to the default repository
- Original file paths will be preserved for backwards compatibility

#### Configuration Changes
- Repository configurations can be managed via API or configuration files
- Default repository assignment for new tasks
- Configurable sync intervals and push-on-change behavior

### 📋 **Compatibility**

#### Backwards Compatibility
- Existing task file paths are preserved during migration
- Legacy API endpoints continue to work with database-backed storage
- File-based task discovery still supported during transition period

#### Forward Compatibility
- Repository system designed for future enhancements
- Extensible for additional repository types
- Conflict resolution framework for collaborative editing

---

