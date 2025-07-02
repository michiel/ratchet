# Ratchet Update Command Implementation Plan

**Date:** 2025-07-02  
**Status:** Draft  
**Priority:** High  

## Overview

Implement a cross-platform `ratchet update` command that allows users to update their ratchet binary in-place, replacing the current curl-based installation workflow with a native, self-updating capability.

## Current State Analysis

### Existing Installation Method
- One-line curl commands fetch `install.sh` (Linux/macOS) or `install.ps1` (Windows)
- Scripts detect platform/architecture automatically
- Download from GitHub releases API
- Install to `~/.local/bin` by default
- Include PATH verification and setup guidance
- Support custom install directories via environment variables

### Key Features to Preserve
- Cross-platform support (Linux, macOS, Windows)
- Automatic platform/architecture detection
- GitHub releases API integration
- Installation directory flexibility
- Progress indication and error handling
- Verification of downloaded binaries

## Implementation Plan

### Phase 1: Core Update Infrastructure

#### 1.1 Update Command Structure
```rust
// ratchet-cli/src/commands/update.rs
pub struct UpdateCommand {
    pub force: bool,              // Force update even if same version
    pub pre_release: bool,        // Allow pre-release versions
    pub target_version: Option<String>, // Specific version to install
    pub install_dir: Option<PathBuf>,   // Custom installation directory
    pub backup: bool,             // Create backup of current binary
    pub verify: bool,             // Verify downloaded binary integrity
}
```

#### 1.2 Version Management Module
```rust
// ratchet-core/src/version.rs
pub struct VersionInfo {
    pub current: String,
    pub latest: String,
    pub needs_update: bool,
    pub release_notes: Option<String>,
}

pub trait VersionManager {
    async fn get_current_version(&self) -> Result<String>;
    async fn get_latest_version(&self, include_prerelease: bool) -> Result<ReleaseInfo>;
    async fn compare_versions(&self, current: &str, latest: &str) -> VersionComparison;
}
```

#### 1.3 Download and Installation Module
```rust
// ratchet-core/src/updater.rs
pub struct UpdateManager {
    pub github_client: GitHubClient,
    pub platform_detector: PlatformDetector,
    pub binary_manager: BinaryManager,
}

pub trait Updater {
    async fn check_for_updates(&self) -> Result<UpdateInfo>;
    async fn download_release(&self, release: &ReleaseInfo) -> Result<PathBuf>;
    async fn verify_binary(&self, path: &Path) -> Result<()>;
    async fn install_binary(&self, source: &Path, target: &Path) -> Result<()>;
    async fn create_backup(&self, current: &Path) -> Result<PathBuf>;
}
```

### Phase 2: Platform-Specific Implementation

#### 2.1 Platform Detection (Enhanced)
- Reuse existing logic from install scripts
- Support for additional architectures (ARM64, etc.)
- Handle special cases (WSL, Docker containers)
- Detect package managers for alternative installation paths

#### 2.2 Binary Management
- **Linux/macOS**: Replace running binary (handle file locking)
- **Windows**: Handle `.exe` extension and Windows-specific locking
- **Cross-platform**: Atomic replacement using temp files and moves

#### 2.3 Self-Update Mechanism
```rust
pub struct SelfUpdater {
    current_executable: PathBuf,
    temp_dir: PathBuf,
    backup_dir: PathBuf,
}

impl SelfUpdater {
    // Handle the complexity of replacing a running binary
    pub async fn replace_self(&self, new_binary: &Path) -> Result<()> {
        // 1. Create backup of current binary
        // 2. Download new binary to temp location
        // 3. Verify new binary integrity
        // 4. Atomic replacement (platform-specific)
        // 5. Cleanup temp files
    }
}
```

### Phase 3: Integration and User Experience

#### 3.1 Command Line Interface
```bash
# Basic update check and install
ratchet update

# Check for updates without installing
ratchet update --check-only

# Force update even if same version
ratchet update --force

# Update to specific version
ratchet update --version=v1.2.3

# Update with backup
ratchet update --backup

# Update from pre-release channel
ratchet update --pre-release

# Custom installation directory
ratchet update --install-dir=/opt/bin

# Dry run - show what would be updated
ratchet update --dry-run
```

#### 3.2 Configuration Integration
```yaml
# config.yaml - update settings
update:
  check_interval_hours: 24        # Auto-check frequency
  auto_update: false              # Automatic updates
  include_prerelease: false       # Pre-release versions
  backup_count: 3                 # Number of backups to keep
  install_dir: "~/.local/bin"     # Default installation directory
  github:
    api_url: "https://api.github.com"
    repo: "ratchet-runner/ratchet"
    timeout_seconds: 30
```

#### 3.3 Progress and Feedback
```rust
pub struct UpdateProgress {
    pub stage: UpdateStage,
    pub progress: f64,              // 0.0 to 1.0
    pub message: String,
    pub bytes_downloaded: u64,
    pub total_bytes: Option<u64>,
}

pub enum UpdateStage {
    CheckingVersion,
    DownloadingRelease,
    VerifyingBinary,
    CreatingBackup,
    InstallingBinary,
    CleaningUp,
    Complete,
}
```

### Phase 4: Security and Reliability

#### 4.1 Binary Verification
- SHA256 checksum validation against GitHub releases
- Digital signature verification (if available)
- Size validation to prevent truncated downloads
- Platform-specific executable validation

#### 4.2 Rollback Capability
- Automatic backup creation before updates
- `ratchet update --rollback` command
- Backup rotation and cleanup
- Recovery from failed updates

#### 4.3 Network and Error Handling
- Retry logic with exponential backoff
- Resume interrupted downloads
- Timeout configuration
- Offline mode detection
- Rate limiting for GitHub API

### Phase 5: Advanced Features

#### 5.1 Update Channels
```rust
pub enum UpdateChannel {
    Stable,         // Latest stable release
    Beta,           // Beta releases
    Nightly,        // Development builds
    Custom(String), // Custom release tag pattern
}
```

#### 5.2 Automatic Update Checks
```rust
// Background update checking
pub struct UpdateChecker {
    pub interval: Duration,
    pub last_check: SystemTime,
    pub notification_handler: Box<dyn NotificationHandler>,
}

// Integration with existing commands
impl UpdateChecker {
    pub async fn background_check(&self) -> Result<Option<UpdateInfo>> {
        // Non-blocking update check during normal operations
        // Respect user preferences for auto-checking
    }
}
```

#### 5.3 Integration with Package Managers
- Detect installation via package managers (brew, apt, etc.)
- Provide appropriate guidance for package manager updates
- Handle conflicts between self-update and package manager

## Implementation Timeline

### Week 1: Core Infrastructure
- [ ] Create update command structure in `ratchet-cli`
- [ ] Implement version management module
- [ ] Add GitHub API client for releases
- [ ] Basic platform detection and binary management

### Week 2: Platform-Specific Features  
- [ ] Implement self-update mechanism for each platform
- [ ] Add binary verification and integrity checks
- [ ] Create backup and rollback functionality
- [ ] Handle platform-specific edge cases

### Week 3: User Experience and Integration
- [ ] Complete CLI interface and argument parsing
- [ ] Add configuration file integration
- [ ] Implement progress reporting and user feedback
- [ ] Add comprehensive error handling and recovery

### Week 4: Testing and Polish
- [ ] Cross-platform testing on Linux, macOS, Windows
- [ ] Integration tests with actual GitHub releases
- [ ] Performance optimization and network resilience
- [ ] Documentation and usage examples

## Technical Considerations

### Dependencies
```toml
[dependencies]
# HTTP client for GitHub API and downloads
reqwest = { version = "0.11", features = ["json", "stream"] }

# JSON parsing for GitHub API responses
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# File operations and path handling
tokio = { version = "1.0", features = ["fs", "process"] }

# Cryptographic verification
sha2 = "0.10"

# Progress reporting
indicatif = "0.17"

# Semver version comparison
semver = "1.0"
```

### Error Handling
```rust
#[derive(thiserror::Error, Debug)]
pub enum UpdateError {
    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),
    
    #[error("Version comparison failed: {0}")]
    VersionError(String),
    
    #[error("Binary verification failed: {0}")]
    VerificationError(String),
    
    #[error("Installation failed: {0}")]
    InstallationError(String),
    
    #[error("Permission denied: {0}")]
    PermissionError(String),
    
    #[error("Backup creation failed: {0}")]
    BackupError(String),
}
```

### Testing Strategy
1. **Unit Tests**: Core logic for version comparison, platform detection
2. **Integration Tests**: GitHub API interaction, file operations
3. **End-to-End Tests**: Full update workflow on each platform
4. **Mock Tests**: Simulate network failures, permission errors
5. **Performance Tests**: Large binary downloads, concurrent updates

## Configuration Examples

### Basic Configuration
```yaml
update:
  check_interval_hours: 24
  auto_update: false
  backup_count: 3
```

### Advanced Configuration
```yaml
update:
  channel: "stable"
  check_interval_hours: 6
  auto_update: false
  include_prerelease: false
  backup_count: 5
  install_dir: "~/.local/bin"
  
  github:
    api_url: "https://api.github.com"
    repo: "ratchet-runner/ratchet"
    timeout_seconds: 60
    retry_attempts: 3
    
  verification:
    verify_checksums: true
    verify_signatures: false
    allow_downgrades: false
    
  notifications:
    update_available: true
    update_complete: true
    update_failed: true
```

## Success Metrics

1. **Functionality**: Successfully update binary on all supported platforms
2. **Reliability**: Handle network failures, permission errors gracefully
3. **User Experience**: Clear progress indication and helpful error messages
4. **Performance**: Fast download and installation process
5. **Security**: Proper verification of downloaded binaries
6. **Backward Compatibility**: Maintain existing installation script functionality

## Future Enhancements

1. **Delta Updates**: Download only changed portions of the binary
2. **Background Updates**: Automatic updates with user approval
3. **Telemetry**: Optional usage reporting for update success/failure rates
4. **Plugin Updates**: Extend to update additional components
5. **Enterprise Features**: Centralized update management, policy controls

## Conclusion

The `ratchet update` command will provide a native, cross-platform solution for keeping ratchet installations current. By implementing this feature, we eliminate the dependency on external curl-based installation scripts while providing users with a more reliable and feature-rich update experience.

The implementation preserves all current installation script functionality while adding advanced features like backup/rollback, verification, and configuration-driven behavior. The modular design ensures maintainability and extensibility for future enhancements.