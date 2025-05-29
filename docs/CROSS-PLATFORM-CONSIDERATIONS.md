# Cross-Platform Considerations for Ratchet

## Overview

Ratchet is designed with cross-platform compatibility in mind, supporting Windows, macOS, and Linux operating systems. This document outlines the architectural decisions that enable this compatibility and provides guidance for developers and users working across different platforms.

## Platform Support Status

| Platform | Architecture | Status | Testing |
|----------|-------------|---------|---------|
| Linux x86_64 | `x86_64-unknown-linux-gnu` | ✅ Fully Supported | ✅ CI/CD Tested |
| macOS Intel | `x86_64-apple-darwin` | ✅ Fully Supported | 🟡 Manual Testing |
| macOS Apple Silicon | `aarch64-apple-darwin` | ✅ Fully Supported | 🟡 Manual Testing |
| Windows x64 | `x86_64-pc-windows-msvc` | ✅ Fully Supported | 🟡 Manual Testing |

## Core Architecture Components

### 1. JavaScript Engine - Boa

Ratchet uses the **Boa JavaScript engine** (v0.17), a pure Rust implementation that provides consistent JavaScript execution across all platforms:

- **No external dependencies**: Unlike Node.js-based solutions, Boa is embedded directly
- **Consistent behavior**: Same ECMAScript implementation on all platforms
- **Memory safety**: Rust's guarantees apply equally across platforms
- **No native extensions**: Avoids platform-specific compilation issues

### 2. Process Management

Worker processes are managed using Tokio's cross-platform process APIs:

```rust
// Platform-agnostic process spawning
let current_exe = std::env::current_exe()?;
let mut cmd = Command::new(&current_exe);
cmd.arg("--worker")
   .arg("--worker-id")
   .arg(&worker_id)
   .stdin(std::process::Stdio::piped())
   .stdout(std::process::Stdio::piped())
   .stderr(std::process::Stdio::piped())
   .kill_on_drop(true);
```

**Key features:**
- Uses the current executable path for worker spawning
- Standard I/O pipes work identically across platforms
- Process lifecycle management handled by Tokio

### 3. Inter-Process Communication (IPC)

Ratchet uses a simple, portable IPC mechanism:

- **JSON over stdin/stdout**: Text-based protocol that works everywhere
- **No platform-specific IPC**: Avoids Unix sockets, Windows named pipes, etc.
- **Serde serialization**: Consistent data encoding across platforms

Example message flow:
```
Coordinator -> Worker: {"ExecuteTask": {"job_id": 1, "task_id": 2, ...}}
Worker -> Coordinator: {"TaskResult": {"job_id": 1, "result": {...}}}
```

### 4. File System Operations

All file operations use Rust's standard library abstractions:

```rust
use std::path::{Path, PathBuf};

// Automatic path separator handling
let task_path = PathBuf::from("tasks").join("my-task").join("main.js");
// Results in: "tasks/my-task/main.js" on Unix
// Results in: "tasks\my-task\main.js" on Windows
```

**Best practices:**
- Always use `PathBuf` and `Path` for file paths
- Use `join()` instead of string concatenation
- Avoid hardcoded path separators
- Handle case-sensitivity differences in application logic

### 5. File System Watcher

The `notify` crate (v6.1) provides platform-specific implementations:

| Platform | Backend | Features |
|----------|---------|----------|
| Windows | `ReadDirectoryChangesW` | Efficient directory monitoring |
| macOS | `FSEvents` | Low-latency file system events |
| Linux | `inotify` | Kernel-level file monitoring |

**Configuration example:**
```yaml
registry:
  sources:
    - name: "local-tasks"
      uri: "file://./tasks"
      config:
        watch: true  # Automatically uses platform backend
```

### 6. Database - SQLite

SQLite provides consistent behavior across platforms:

- **File-based storage**: Works with platform file systems
- **Sea-ORM abstraction**: Hides platform differences
- **No external database server**: Simplifies deployment

### 7. HTTP Client

Uses `reqwest` with `rustls` for cross-platform HTTPS:

- **Pure Rust TLS**: No OpenSSL dependency issues
- **Consistent certificate handling**: Same behavior everywhere
- **Proxy support**: Respects system proxy settings

## Platform-Specific Considerations

### Windows

#### File Paths
- **Case-insensitive**: `Task.js` and `task.js` are the same file
- **Reserved names**: Avoid `CON`, `PRN`, `AUX`, `NUL`, etc.
- **Path length**: Traditional 260 character limit (can be extended)
- **Separators**: Backslashes (`\`) used, but forward slashes (`/`) also work

#### Process Management
- **Process groups**: Different from Unix process groups
- **Signal handling**: Limited compared to Unix signals
- **Console behavior**: May spawn visible console windows

#### Development Tips
```powershell
# Build on Windows
cargo build --release

# Run with logging
$env:RUST_LOG="ratchet=debug"
./target/release/ratchet.exe serve --config config.yaml
```

### macOS

#### File System
- **Case-insensitive by default**: But can be case-sensitive
- **Extended attributes**: May include `.DS_Store` files
- **Permissions**: Unix-style with additional macOS attributes

#### Security
- **Gatekeeper**: Unsigned binaries may require approval
- **Code signing**: Required for distribution outside App Store
- **Notarization**: Recommended for user trust

#### Development Tips
```bash
# Build for Intel Mac
cargo build --release --target x86_64-apple-darwin

# Build for Apple Silicon
cargo build --release --target aarch64-apple-darwin

# Universal binary (requires cargo-lipo)
cargo lipo --release
```

### Linux

#### File System
- **Case-sensitive**: `Task.js` and `task.js` are different files
- **Hidden files**: Start with `.` (e.g., `.env`)
- **Permissions**: Standard Unix permissions apply

#### Process Management
- **Signals**: Full Unix signal support
- **Process groups**: Standard Unix behavior
- **Resource limits**: Can be configured via ulimit

#### Distribution
```bash
# Build static binary (musl)
cargo build --release --target x86_64-unknown-linux-musl

# Create AppImage (requires appimagetool)
./create-appimage.sh
```

## Building for Multiple Platforms

### Cross-Compilation Setup

1. **Install targets:**
```bash
rustup target add x86_64-pc-windows-msvc
rustup target add x86_64-apple-darwin
rustup target add aarch64-apple-darwin
```

2. **Install cross-compilation tools:**
- Windows: `mingw-w64` or Visual Studio Build Tools
- macOS: `osxcross` or actual Mac hardware

### Build Commands

```bash
# Windows from Linux/macOS
cargo build --release --target x86_64-pc-windows-msvc

# macOS from Linux (requires osxcross)
cargo build --release --target x86_64-apple-darwin

# Linux static binary
cargo build --release --target x86_64-unknown-linux-musl
```

### CI/CD Configuration

Example GitHub Actions workflow:
```yaml
strategy:
  matrix:
    include:
      - os: ubuntu-latest
        target: x86_64-unknown-linux-gnu
      - os: windows-latest
        target: x86_64-pc-windows-msvc
      - os: macos-latest
        target: x86_64-apple-darwin
      - os: macos-latest
        target: aarch64-apple-darwin
```

## Testing Across Platforms

### Platform-Specific Test Considerations

1. **File system tests:**
   - Test both case-sensitive and case-insensitive scenarios
   - Handle different path length limitations
   - Test Unicode filenames

2. **Process tests:**
   - Verify worker process spawning
   - Test signal handling (where applicable)
   - Validate IPC communication

3. **Integration tests:**
   ```rust
   #[cfg(test)]
   mod tests {
       #[test]
       #[cfg(target_os = "windows")]
       fn test_windows_paths() {
           // Windows-specific path tests
       }
       
       #[test]
       #[cfg(unix)]
       fn test_unix_permissions() {
           // Unix-specific permission tests
       }
   }
   ```

## Deployment Considerations

### Binary Distribution

1. **Single executable**: Ratchet compiles to a single binary
2. **No runtime dependencies**: Beyond system libraries
3. **Configuration files**: Use platform-agnostic YAML

### Installation Paths

| Platform | Recommended Installation Path |
|----------|------------------------------|
| Windows | `C:\Program Files\Ratchet\` or `%LOCALAPPDATA%\Ratchet\` |
| macOS | `/Applications/Ratchet.app/` or `/usr/local/bin/` |
| Linux | `/usr/local/bin/` or `/opt/ratchet/` |

### Configuration Locations

| Platform | Config Path |
|----------|-------------|
| Windows | `%APPDATA%\Ratchet\config.yaml` |
| macOS | `~/Library/Application Support/Ratchet/config.yaml` |
| Linux | `~/.config/ratchet/config.yaml` or `/etc/ratchet/config.yaml` |

## Known Limitations and Workarounds

### Windows
- **Long path support**: Enable via registry or manifest
- **Console windows**: Use `#![windows_subsystem = "windows"]` for GUI apps
- **File locking**: More restrictive than Unix systems

### macOS
- **Unsigned binaries**: Users may need to right-click → Open
- **Quarantine attribute**: Can be removed with `xattr -d com.apple.quarantine`

### All Platforms
- **Relative paths**: Always resolve to absolute paths for consistency
- **Time zones**: Use UTC internally, convert for display
- **Line endings**: Handle both LF and CRLF in text files

## Future Improvements

1. **Platform-specific optimizations**: Leverage unique OS features
2. **Native installers**: MSI (Windows), DMG (macOS), DEB/RPM (Linux)
3. **Platform-specific tests**: Expand CI/CD coverage
4. **Performance profiling**: Per-platform optimization opportunities

## Contributing

When contributing platform-specific code:

1. Use conditional compilation sparingly:
   ```rust
   #[cfg(target_os = "windows")]
   fn platform_specific_function() { }
   ```

2. Prefer cross-platform abstractions:
   ```rust
   // Good: Let std handle platform differences
   use std::path::PathBuf;
   
   // Avoid: Platform-specific paths
   #[cfg(windows)]
   const CONFIG_PATH: &str = "C:\\config";
   ```

3. Test on multiple platforms or use CI/CD

4. Document platform-specific behavior

## Resources

- [Rust Platform Support](https://doc.rust-lang.org/nightly/rustc/platform-support.html)
- [Cross-compilation Guide](https://rust-lang.github.io/rustup/cross-compilation.html)
- [notify Crate Documentation](https://docs.rs/notify/)
- [Tokio Process Documentation](https://docs.rs/tokio/latest/tokio/process/)