# Git+HTTP Task Repository Design Proposal

## Overview

This document proposes adding Git repositories accessed over HTTP/HTTPS as a source for task repositories in Ratchet. This approach leverages existing Git infrastructure (GitHub, GitLab, Bitbucket, etc.) to enable distributed task management with built-in versioning, authentication, and collaboration features. Organizations can use their existing Git repositories to distribute tasks while maintaining security, caching, and validation.

## Current Architecture Analysis

### Existing Registry System

The current registry system in `ratchet-registry` supports:
- **Filesystem sources**: Local directories with task discovery
- **HTTP sources** (partially implemented): Basic structure exists but discovery/loading not implemented
- **Modular loader pattern**: `TaskLoader` trait with specialized implementations
- **Caching layer**: LRU cache with TTL for task content
- **Configuration**: Comprehensive source configuration in `ratchet-config`

### Current Limitations

1. **Git loader incomplete**: Only stub implementation exists in `ratchet-config`
2. **No Git repository integration**: No actual Git operations for cloning/pulling
3. **Limited authentication**: Git auth types defined but not implemented
4. **No Git-specific caching**: Missing efficient Git-aware caching strategies
5. **No branch/tag support**: No support for different Git refs

## Proposed Git+HTTP Task Repository Design

### 1. Git Repository Structure for Tasks

#### Repository Layout
```
task-repository/
├── .ratchet/                    # Repository metadata
│   ├── registry.yaml           # Repository configuration
│   └── index.json              # Task index for fast discovery
├── tasks/                      # Task directories
│   ├── weather-api/
│   │   ├── metadata.json
│   │   ├── main.js
│   │   ├── input.schema.json
│   │   ├── output.schema.json
│   │   └── tests/
│   │       └── test-001.json
│   ├── data-processor/
│   │   ├── metadata.json
│   │   ├── main.js
│   │   └── README.md
│   └── utils/
│       └── common-functions/
├── collections/                 # Task collections/bundles
│   ├── data-pipeline.yaml      # Collection of related tasks
│   └── monitoring-tasks.yaml
├── templates/                   # Task templates
│   ├── basic-api-task/
│   └── data-transformation/
└── README.md                   # Repository documentation
```

#### Repository Metadata (.ratchet/registry.yaml)
```yaml
name: "Corporate Task Repository"
description: "Internal tasks for data processing and automation"
version: "2.1.0"
maintainers:
  - name: "Data Team"
    email: "data-team@corp.com"
categories:
  - "data-processing"
  - "api-integration"
  - "monitoring"
tags:
  - "production"
  - "enterprise"
ratchet_version: ">=0.6.0"
discovery:
  auto_index: true
  include_patterns:
    - "tasks/**"
    - "collections/**"
  exclude_patterns:
    - "**/.git/**"
    - "**/node_modules/**"
```

#### Task Index (.ratchet/index.json)
```json
{
  "generated_at": "2024-03-01T14:20:00Z",
  "repository": {
    "name": "Corporate Task Repository",
    "version": "2.1.0",
    "commit": "abc123def456"
  },
  "tasks": [
    {
      "name": "weather-api",
      "version": "1.2.0",
      "path": "tasks/weather-api",
      "uuid": "550e8400-e29b-41d4-a716-446655440000",
      "description": "Fetch weather data from external APIs",
      "tags": ["weather", "api"],
      "last_modified": "2024-02-15T10:30:00Z",
      "checksum": "sha256:d4b2f7e8c1a9..."
    }
  ],
  "collections": [
    {
      "name": "data-pipeline",
      "path": "collections/data-pipeline.yaml",
      "description": "Complete data processing pipeline",
      "tasks": ["data-extractor", "data-transformer", "data-loader"]
    }
  ]
}
```

### 2. Enhanced Configuration Schema

#### Git Repository Source Configuration
```yaml
registry:
  sources:
    - name: "corporate-tasks"
      uri: "https://github.com/corp/ratchet-tasks.git"
      source_type: "git"
      enabled: true
      auth_name: "github_token"
      polling_interval: "10m"
      config:
        git:
          # Git reference (branch, tag, or commit)
          ref: "main"                    # or "v1.2.0" or "abc123def"
          
          # Subdirectory within repository (optional)
          subdirectory: "production-tasks"
          
          # Clone behavior
          shallow: true                  # Shallow clone for performance
          depth: 1                       # Clone depth for shallow clones
          
          # Sync strategy
          sync_strategy: "fetch"         # "clone", "fetch", or "pull"
          cleanup_on_error: true
          
          # Content validation
          verify_signatures: false       # Verify Git commit signatures
          allowed_refs:                  # Restrict to specific refs
            - "main"
            - "release/*"
            - "v*"
          
          # Performance settings
          timeout: "300s"                # Git operation timeout
          max_repo_size: "100MB"
          
          # Caching
          local_cache_path: "/tmp/ratchet-git-cache"
          cache_ttl: "1h"
          keep_history: false            # Keep Git history or just working tree

  # Authentication configurations
  auth:
    github_token:
      type: "git_token"
      token: "${GITHUB_TOKEN}"
      
    gitlab_ssh:
      type: "ssh_key"
      private_key_path: "/home/user/.ssh/id_rsa"
      passphrase: "${SSH_PASSPHRASE}"
      
    github_app:
      type: "github_app"
      app_id: "${GITHUB_APP_ID}"
      private_key_path: "/etc/github-app.pem"
      installation_id: "${GITHUB_INSTALLATION_ID}"
```

#### Git Provider-Specific Configuration Examples

##### GitHub Configuration
```yaml
registry:
  sources:
    - name: "github-public-tasks"
      uri: "https://github.com/michiel/ratchet-repo-samples"
      source_type: "git"
      config:
        git:
          ref: "main"
          shallow: true
          depth: 1
          
    - name: "github-private-tasks"
      uri: "https://github.com/corp/private-tasks.git"
      source_type: "git"
      auth_name: "github_pat"
      config:
        git:
          ref: "production"
          subdirectory: "approved-tasks"
          verify_signatures: true

  auth:
    github_pat:
      type: "git_token"
      token: "${GITHUB_TOKEN}"
```

##### GitLab Configuration
```yaml
registry:
  sources:
    - name: "gitlab-tasks"
      uri: "https://gitlab.com/corp/automation-tasks.git"
      source_type: "git"
      auth_name: "gitlab_deploy_key"
      config:
        git:
          ref: "stable"
          sync_strategy: "fetch"
          
  auth:
    gitlab_deploy_key:
      type: "ssh_key"
      private_key_path: "/etc/deploy-keys/gitlab.pem"
```

##### Bitbucket Configuration
```yaml
registry:
  sources:
    - name: "bitbucket-tasks"
      uri: "https://bitbucket.org/corp/ratchet-tasks.git"
      source_type: "git"
      auth_name: "bitbucket_app_password"
      
  auth:
    bitbucket_app_password:
      type: "basic"
      username: "service-account"
      password: "${BITBUCKET_APP_PASSWORD}"
```

##### Self-Hosted Git Configuration
```yaml
registry:
  sources:
    - name: "internal-git"
      uri: "https://git.internal.corp/automation/tasks.git"
      source_type: "git"
      auth_name: "internal_cert"
      config:
        git:
          ref: "production"
          verify_ssl_certs: false  # For self-signed certificates
          
  auth:
    internal_cert:
      type: "client_certificate"
      cert_path: "/etc/ssl/client.crt"
      key_path: "/etc/ssl/client.key"
      ca_cert_path: "/etc/ssl/internal-ca.crt"
```

### 3. Implementation Architecture

#### Enhanced TaskLoader Interface
```rust
#[async_trait]
pub trait TaskLoader: Send + Sync {
    async fn discover_tasks(&self, source: &TaskSource) -> Result<Vec<DiscoveredTask>>;
    async fn load_task(&self, task_ref: &TaskReference) -> Result<TaskDefinition>;
    async fn supports_source(&self, source: &TaskSource) -> bool;
    
    // New methods for Git sources
    async fn validate_source(&self, source: &TaskSource) -> Result<SourceValidation>;
    async fn get_source_info(&self, source: &TaskSource) -> Result<SourceInfo>;
    async fn check_for_updates(&self, source: &TaskSource) -> Result<UpdateInfo>;
    async fn sync_repository(&self, source: &TaskSource) -> Result<SyncResult>;
}
```

#### Git Loader Implementation Structure
```rust
pub struct GitLoader {
    git_client: Arc<GitClient>,
    cache: Arc<GitRepositoryCache>,
    config: GitLoaderConfig,
    auth_manager: Arc<GitAuthManager>,
}

impl GitLoader {
    pub async fn discover_tasks(&self, source: &TaskSource) -> Result<Vec<DiscoveredTask>> {
        // 1. Ensure repository is cloned/synced
        // 2. Read .ratchet/index.json if available (fast path)
        // 3. Otherwise scan tasks/ directory recursively
        // 4. Parse metadata.json for each task
        // 5. Build DiscoveredTask entries
        // 6. Cache results with Git commit hash
        // 7. Return discovered tasks
    }
    
    pub async fn load_task(&self, task_ref: &TaskReference) -> Result<TaskDefinition> {
        // 1. Ensure repository is up to date
        // 2. Navigate to task directory
        // 3. Load all task files (metadata.json, main.js, schemas)
        // 4. Validate task structure
        // 5. Build TaskDefinition with Git metadata
        // 6. Cache with Git commit hash
        // 7. Return task definition
    }
    
    async fn sync_repository(&self, source: &TaskSource) -> Result<SyncResult> {
        // Git operations: clone, fetch, pull based on strategy
        // Handle authentication, SSL, and error recovery
    }
    
    async fn get_repository_path(&self, source: &TaskSource) -> Result<PathBuf> {
        // Compute local cache path for repository
    }
    
    async fn check_git_updates(&self, source: &TaskSource) -> Result<bool> {
        // Check if remote has new commits
    }
    
    async fn scan_tasks_directory(&self, repo_path: &Path, subdir: Option<&str>) -> Result<Vec<DiscoveredTask>> {
        // Recursive directory scanning for tasks
    }
    
    async fn load_registry_index(&self, repo_path: &Path) -> Result<Option<RegistryIndex>> {
        // Load .ratchet/index.json for fast discovery
    }
}

pub struct GitClient {
    config: GitClientConfig,
}

impl GitClient {
    pub async fn clone_repository(&self, url: &str, local_path: &Path, config: &GitCloneConfig) -> Result<()> {
        // Use git2 or libgit2 to clone repositories
    }
    
    pub async fn fetch_updates(&self, repo_path: &Path, remote_ref: &str) -> Result<FetchResult> {
        // Fetch latest changes from remote
    }
    
    pub async fn checkout_ref(&self, repo_path: &Path, git_ref: &str) -> Result<()> {
        // Checkout specific branch, tag, or commit
    }
    
    pub async fn get_current_commit(&self, repo_path: &Path) -> Result<String> {
        // Get current HEAD commit hash
    }
    
    pub async fn verify_signatures(&self, repo_path: &Path) -> Result<bool> {
        // Verify Git commit signatures if enabled
    }
}

pub struct GitAuthManager {
    auth_configs: HashMap<String, GitAuthConfig>,
}

impl GitAuthManager {
    pub fn setup_credentials(&self, auth_name: &str, git_config: &mut git2::Config) -> Result<()> {
        // Configure Git credentials based on auth type
        // Handle tokens, SSH keys, certificates
    }
}
```

### 4. Security Considerations

#### Git Authentication and Authorization
- **Multiple auth types**: Personal access tokens, SSH keys, GitHub Apps, Deploy keys
- **Environment variable support**: Secure credential management with `${VAR}` syntax
- **Per-repository authentication**: Different credentials for different Git providers
- **Credential caching**: Secure in-memory credential storage

#### Git-Specific Security
```yaml
security:
  # Repository access control
  allowed_hosts:
    - "github.com"
    - "gitlab.com" 
    - "git.internal.corp"
  
  # Ref restrictions
  allowed_refs:
    - "main"
    - "master"
    - "release/*"
    - "v*"
  blocked_refs:
    - "experimental/*"
    - "dev/*"
  
  # Commit verification
  verify_signatures: true
  trusted_signers:
    - "dev-team@corp.com"
    - "automation@corp.com"
  
  # Content validation
  max_repo_size: "100MB"
  max_file_size: "10MB"
  scan_for_secrets: true
  
  # Clone restrictions
  shallow_only: true
  max_depth: 10
  timeout: "300s"
```

#### Repository Content Security
- **Commit signature verification**: GPG/SSH signature validation
- **Content scanning**: Scan for secrets, malicious code patterns
- **Size limits**: Repository and individual file size restrictions
- **Ref validation**: Restrict to approved branches/tags only
- **Host allowlisting**: Only allow trusted Git hosting providers

#### Network Security
- **TLS/SSL verification**: Certificate validation for HTTPS Git operations
- **SSH host key verification**: Validate SSH host keys for Git over SSH
- **Proxy support**: Corporate proxy and firewall compatibility
- **Timeout controls**: Prevent hanging Git operations
- **Rate limiting**: Respect Git provider rate limits

### 5. Git-Aware Caching Strategy

#### Multi-Level Git Caching
```rust
pub struct GitRepositoryCache {
    // Local Git repositories (bare repos for efficiency)
    repo_cache: Arc<LruCache<String, GitRepository>>,
    
    // Task metadata cache (invalidated by Git commit hash)
    task_cache: Arc<LruCache<String, TaskDefinition>>,
    
    // Discovery cache (task lists per repository)
    discovery_cache: Arc<LruCache<String, Vec<DiscoveredTask>>>,
    
    // Git object cache for frequently accessed files
    object_cache: Arc<LruCache<String, Vec<u8>>>,
}
```

#### Git-Based Cache Invalidation
- **Commit-based**: Cache entries tagged with Git commit hash
- **Ref tracking**: Monitor changes to tracked branches/tags
- **Incremental updates**: Only fetch changed objects since last sync
- **Repository-level TTL**: Force periodic full syncs regardless of changes
- **Manual refresh**: Support force refresh of specific repositories

#### Efficient Git Operations
```yaml
caching:
  # Repository-level caching
  local_cache_path: "/var/cache/ratchet/git-repos"
  keep_bare_repos: true           # Use bare repos for efficiency
  cleanup_interval: "24h"         # Clean up unused repos
  max_repo_age: "7d"             # Remove repos not accessed in 7 days
  
  # Object-level caching  
  cache_git_objects: true         # Cache individual Git objects
  object_cache_size: "100MB"
  
  # Task-level caching
  task_cache_ttl: "1h"           # Task definition cache TTL
  discovery_cache_ttl: "30m"     # Task discovery cache TTL
  
  # Sync behavior
  check_remote_interval: "5m"    # How often to check for remote changes
  fetch_strategy: "minimal"      # "full", "minimal", or "on-demand"
```

### 6. Git-Specific Error Handling and Resilience

#### Git Operation Retry Strategy
```rust
pub struct GitRetryConfig {
    pub max_attempts: u32,
    pub backoff: BackoffStrategy,
    pub base_delay: Duration,
    pub max_delay: Duration,
    pub jitter: bool,
    pub retryable_git_errors: Vec<GitErrorType>,
}

pub enum GitErrorType {
    NetworkTimeout,
    RemoteUnavailable,
    AuthenticationFailure,
    RepositoryCorrupted,
    DiskSpaceError,
}
```

#### Git-Aware Circuit Breaker
- **Per-repository circuit breakers**: Isolate failures to specific repos
- **Authentication failure handling**: Separate circuit for auth errors
- **Network vs. repository failures**: Different thresholds for different error types
- **Recovery strategies**: Automatic retry, manual intervention, or fallback to cache

#### Git Resilience Patterns
```yaml
resilience:
  # Circuit breaker configuration
  circuit_breaker:
    failure_threshold: 5
    recovery_timeout: "60s"
    success_threshold: 3
    
  # Retry configuration
  retry:
    max_attempts: 3
    base_delay: "2s"
    max_delay: "30s"
    backoff: "exponential"
    retryable_errors:
      - "network_timeout"
      - "remote_unavailable"
      - "temporary_auth_failure"
  
  # Fallback strategies
  fallback:
    use_cached_repo: true          # Use local cached copy
    use_last_known_good: true      # Fall back to last successful sync
    partial_failure_ok: true       # Continue with available tasks
    
  # Health monitoring
  health_check:
    interval: "30s"
    timeout: "10s"
    max_failures: 3
```

#### Graceful Degradation for Git Sources
- **Offline mode**: Use locally cached repositories when remote unavailable
- **Stale content serving**: Serve tasks from last successful sync
- **Partial repository failures**: Continue with other repositories if one fails
- **Automatic recovery**: Resume normal operation when connectivity restored

### 7. Implementation Plan

#### Phase 1: Core Git Integration
1. **Git client library**: Add `git2` dependency and basic Git operations
2. **GitLoader implementation**: Core task discovery and loading from Git repos
3. **Basic authentication**: Support for HTTPS tokens and SSH keys
4. **Local repository caching**: Efficient local Git repository management
5. **Configuration integration**: Update config schema for Git sources

**Dependencies to add:**
- `git2` - Rust Git library
- `url` - URL parsing for Git URLs
- `dirs` - Cross-platform directory detection for cache

#### Phase 2: Advanced Git Features
1. **Repository index optimization**: `.ratchet/index.json` for fast discovery
2. **Branch/tag support**: Support for different Git refs and versioning
3. **Signature verification**: GPG commit signature validation
4. **Enhanced caching**: Multi-level caching with Git-aware invalidation
5. **Git provider optimization**: Provider-specific optimizations (GitHub, GitLab)

#### Phase 3: Enterprise Git Features
1. **Git LFS support**: Large File Storage for binary task assets
2. **Monorepo support**: Efficient handling of large repositories
3. **Webhook integration**: Real-time updates via Git webhooks
4. **Advanced security**: Content scanning, secret detection
5. **Administrative tools**: CLI commands for Git repository management

#### Phase 4: Integration and Polish
1. **MCP integration**: Expose Git sources via MCP tools
2. **GraphQL/REST APIs**: Add Git-specific endpoints
3. **Monitoring and metrics**: Git operation metrics and health checks
4. **Documentation**: Comprehensive documentation and examples
5. **Testing**: Integration tests with real Git repositories

### 8. Real-World Configuration Examples

#### Basic Public Git Repository
```yaml
registry:
  sources:
    - name: "community-tasks"
      uri: "https://github.com/michiel/ratchet-repo-samples"
      source_type: "git"
      enabled: true
      config:
        git:
          ref: "main"
          shallow: true
          depth: 1
          timeout: "60s"
```

#### Enterprise Git Repository with Authentication
```yaml
registry:
  sources:
    - name: "corporate-tasks"
      uri: "https://github.com/corp/ratchet-tasks.git"
      source_type: "git"
      enabled: true
      auth_name: "github_corp"
      polling_interval: "15m"
      config:
        git:
          ref: "production"
          subdirectory: "approved-tasks"
          shallow: true
          verify_signatures: true
          allowed_refs:
            - "production"
            - "release/*"
          caching:
            local_cache_path: "/var/cache/ratchet/corp-tasks"
            task_cache_ttl: "2h"

  auth:
    github_corp:
      type: "git_token"
      token: "${GITHUB_CORP_TOKEN}"
```

#### Multi-Source Setup (Git + Filesystem)
```yaml
registry:
  sources:
    # Community tasks from GitHub
    - name: "github-community"
      uri: "https://github.com/michiel/ratchet-repo-samples"
      source_type: "git"
      enabled: true
      config:
        git:
          ref: "stable"
          
    # Corporate tasks from GitLab
    - name: "gitlab-corporate"
      uri: "https://gitlab.corp.com/automation/ratchet-tasks.git"
      source_type: "git"
      enabled: true
      auth_name: "gitlab_deploy_key"
      config:
        git:
          ref: "main"
          subdirectory: "production"
          
    # Local development tasks
    - name: "local-dev"
      uri: "file://./dev-tasks"
      source_type: "filesystem"
      enabled: true
      config:
        filesystem:
          watch_changes: true

  auth:
    gitlab_deploy_key:
      type: "ssh_key"
      private_key_path: "/etc/ratchet/gitlab-deploy.key"

# Global caching configuration
cache:
  enabled: true
  default_ttl: "1h"
  max_entries: 1000
  cleanup_interval: "6h"
```

#### Production-Ready Configuration
```yaml
registry:
  sources:
    # Primary task repository
    - name: "production-tasks"
      uri: "https://github.com/corp/production-tasks.git"
      source_type: "git"
      enabled: true
      auth_name: "github_app"
      polling_interval: "5m"
      config:
        git:
          ref: "v2.1.0"                    # Pin to specific version
          shallow: true
          verify_signatures: true
          max_repo_size: "50MB"
          timeout: "120s"
          
    # Fallback repository for critical tasks
    - name: "fallback-tasks"
      uri: "https://backup-git.corp.com/critical-tasks.git"
      source_type: "git"
      enabled: true
      auth_name: "internal_cert"
      config:
        git:
          ref: "stable"
          sync_strategy: "fetch"

  auth:
    github_app:
      type: "github_app"
      app_id: "${GITHUB_APP_ID}"
      private_key_path: "/etc/ratchet/github-app.pem"
      installation_id: "${GITHUB_INSTALLATION_ID}"
      
    internal_cert:
      type: "client_certificate"
      cert_path: "/etc/ssl/ratchet-client.crt"
      key_path: "/etc/ssl/ratchet-client.key"

# Security configuration
security:
  allowed_hosts:
    - "github.com"
    - "backup-git.corp.com"
  verify_signatures: true
  max_repo_size: "100MB"
  scan_for_secrets: true

# Resilience configuration
resilience:
  circuit_breaker:
    failure_threshold: 3
    recovery_timeout: "120s"
  retry:
    max_attempts: 5
    base_delay: "2s"
    backoff: "exponential"
  fallback:
    use_cached_repo: true
    partial_failure_ok: true
```

### 9. Benefits and Use Cases

#### For Organizations
- **Git-native workflow**: Leverage existing Git infrastructure and expertise
- **Built-in version control**: Full Git history, branching, and tagging capabilities
- **Access control**: Use existing Git repository permissions and authentication
- **Compliance**: Git commit signatures and audit trails
- **Global distribution**: Leverage Git's distributed nature and CDN capabilities

#### For Developers
- **Familiar tooling**: Use standard Git workflows for task development
- **Collaboration**: Git branching, pull requests, and code review for tasks
- **Version management**: Semantic versioning with Git tags and releases
- **Testing workflows**: Feature branches for task development and testing
- **IDE integration**: Full Git support in development environments

#### For Operations
- **High availability**: Git's distributed nature provides natural redundancy
- **Efficient caching**: Git's object model enables efficient local caching
- **Monitoring**: Git repository health and sync status monitoring
- **Security**: GPG signing, SSH authentication, and repository access controls
- **Disaster recovery**: Git repositories can be easily backed up and restored

#### Git-Specific Advantages
- **Offline capability**: Local Git cache allows offline operation
- **Delta updates**: Git only transfers changed objects, not entire repositories
- **Integrity checking**: Git's content-addressable storage ensures data integrity
- **Branching strategies**: Support for GitFlow, trunk-based development, etc.
- **Integration**: Native integration with CI/CD pipelines and Git hosting platforms

### 10. Migration Path

#### Backward Compatibility
- **Existing filesystem sources**: Continue to work unchanged
- **Configuration migration**: Automatic migration to support Git sources
- **API compatibility**: Maintain existing TaskLoader interface with Git extensions

#### Gradual Git Adoption
1. **Phase 1**: Add Git sources alongside existing filesystem sources
2. **Phase 2**: Create Git repositories for existing task collections
3. **Phase 3**: Migrate high-frequency tasks to Git repositories
4. **Phase 4**: Establish Git as primary distribution method while keeping filesystem for local development

#### Migration Strategies
```yaml
# Migration example: Hybrid filesystem + Git
registry:
  sources:
    # Keep existing local tasks during migration
    - name: "local-legacy"
      uri: "file://./legacy-tasks"
      source_type: "filesystem"
      enabled: true
      
    # Add new Git source
    - name: "git-new"
      uri: "https://github.com/corp/migrated-tasks.git"
      source_type: "git"
      enabled: true
      
    # Gradual migration with Git taking priority
    - name: "git-priority"
      uri: "https://github.com/corp/all-tasks.git"
      source_type: "git"
      enabled: true
      config:
        git:
          ref: "main"
          # Override local tasks with same names
          priority: 100
```

## Conclusion

This design provides a comprehensive foundation for Git+HTTP-based task repositories in Ratchet. By leveraging Git's proven infrastructure and tooling, the implementation provides enterprise-grade capabilities while remaining familiar to developers and operators.

The proposed Git repository system will enable organizations to:
- **Leverage existing Git infrastructure** and expertise
- **Implement robust version control** with full Git capabilities
- **Ensure security and compliance** through Git's built-in features
- **Scale globally** using Git's distributed architecture
- **Integrate seamlessly** with existing development workflows

### Key Advantages Over Custom HTTP APIs

1. **No custom server required**: Use existing Git hosting (GitHub, GitLab, etc.)
2. **Rich tooling ecosystem**: Leverage Git's mature tooling and integrations
3. **Natural versioning**: Git tags and branches provide robust versioning
4. **Efficient synchronization**: Git's delta compression minimizes bandwidth
5. **Offline capability**: Local Git repositories enable offline operation
6. **Developer familiarity**: Standard Git workflows need no additional training

This approach positions Ratchet as a practical, enterprise-ready task execution platform that integrates naturally with existing development infrastructure and workflows. The Git+HTTP approach provides better long-term maintainability and adoption potential compared to custom HTTP APIs.