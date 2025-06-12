# Ratchet Build Options

This document describes the various build configuration options available for Ratchet.

## TLS Configuration

Ratchet uses a **hybrid TLS approach** for optimal security and functionality:

- **HTTP Client Operations**: Pure Rust TLS with `rustls` (ratchet-http, reqwest)
- **Git Repository Access**: OpenSSL through `git2` for HTTPS repository cloning

### Git OpenSSL Options

#### Default: System OpenSSL
```bash
# Uses system-installed OpenSSL libraries
cargo build --features git
```

**Pros**: 
- Fast build times
- Uses system-maintained OpenSSL
- Smaller binary size

**Cons**:
- Requires OpenSSL development libraries installed
- Cross-compilation complexity

#### Optional: Vendored OpenSSL
```bash
# Compiles OpenSSL from source (requires perl with FindBin module)
cargo build --features git-vendored
```

**Pros**:
- No system OpenSSL dependencies required
- Consistent OpenSSL version across platforms
- Better for cross-compilation and static linking

**Cons**:
- Slower build times (compiles OpenSSL from source)
- Larger binary size
- Requires perl with FindBin module installed

### Prerequisites for Vendored OpenSSL

To use the `git-vendored` feature, ensure perl is properly configured:

**Fedora/RHEL/CentOS:**
```bash
# Install perl with required modules
sudo dnf install perl-core perl-FindBin perl-File-Compare perl-File-Copy
```

**Ubuntu/Debian:**
```bash
# Install perl with required modules  
sudo apt-get install perl perl-modules-5.* libfindbin-libs-perl
```

**macOS:**
```bash
# Usually perl is pre-installed with required modules
perl -e "use FindBin;" && echo "Ready for vendored build"
```

**Windows:**
```powershell
# Install Strawberry Perl which includes required modules
# Download from https://strawberryperl.com/
```

### Testing OpenSSL Configuration

Verify your chosen OpenSSL configuration works:

```bash
# Test system OpenSSL (default)
cargo test -p ratchet-registry --features git https_git_tests

# Test vendored OpenSSL (if perl configured)
cargo test -p ratchet-registry --features git-vendored https_git_tests
```

## Feature Flags

### Registry Features
- `git` - Enable Git repository support with system OpenSSL
- `git-vendored` - Enable Git repository support with vendored OpenSSL
- `filesystem` - Local filesystem task loading
- `http` - HTTP-based task loading
- `watcher` - File system watching for auto-reload
- `validation` - JSON schema validation

### Core Features
- `server` - REST and GraphQL APIs
- `mcp-server` - Model Context Protocol for LLM integration  
- `javascript` - JavaScript task execution
- `caching` - Task and HTTP response caching
- `resilience` - Circuit breakers and retry policies

### Example Builds

```bash
# Minimal build (no Git support)
cargo build --no-default-features --features filesystem,http

# Git with system OpenSSL
cargo build --features git

# Git with vendored OpenSSL (cross-compilation friendly)
cargo build --features git-vendored

# Full build with all features
cargo build --features full

# Production build optimized for deployment
cargo build --profile dist --features production
```

## Cross-Platform Considerations

### Linux
- System OpenSSL: Install `openssl-devel` or `libssl-dev` packages
- Vendored OpenSSL: Install `perl-core` package
- Both approaches work well

### macOS  
- System OpenSSL: Uses system libraries (may be older versions)
- Vendored OpenSSL: Recommended for consistent builds
- Perl usually pre-installed with required modules

### Windows
- System OpenSSL: Requires vcpkg or manual OpenSSL installation
- Vendored OpenSSL: Recommended, requires Strawberry Perl
- Consider using the `git-vendored` feature for easier builds

## Performance Impact

| Configuration | Build Time | Binary Size | Runtime Performance |
|---------------|------------|-------------|-------------------|
| System OpenSSL | Fast | Smaller | Excellent |
| Vendored OpenSSL | Slow (first build) | Larger | Excellent |
| No Git | Fastest | Smallest | N/A (no Git) |

## Troubleshooting

### "openssl-sys build failed"
```bash
# Install system OpenSSL development libraries
# Fedora/RHEL: sudo dnf install openssl-devel
# Ubuntu/Debian: sudo apt-get install libssl-dev
```

### "Can't locate FindBin.pm"
```bash
# Install perl modules for vendored builds
# Fedora/RHEL: sudo dnf install perl-core perl-FindBin
# Ubuntu/Debian: sudo apt-get install perl-modules-5.*
```

### "git2 HTTPS clone failed"
```bash
# Verify HTTPS support is enabled
cargo test -p ratchet-registry --features git https_git_tests
```

## Recommendations

### Development
- Use **default features** (`git` with system OpenSSL) for faster builds
- Enable `watcher` feature for auto-reload during development

### Production  
- Consider **`git-vendored`** for consistent deployments
- Use `production` feature profile for optimized builds
- Test both HTTP client (rustls) and Git (OpenSSL) functionality

### CI/CD
- Use **`git-vendored`** to avoid OpenSSL system dependencies
- Cache the target directory to avoid rebuilding OpenSSL
- Test multiple feature combinations to ensure compatibility

---

The hybrid TLS approach provides the optimal balance between security (pure Rust for HTTP) and functionality (mature OpenSSL for Git), with flexible build options to suit different deployment scenarios.