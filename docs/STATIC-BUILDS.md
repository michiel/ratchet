# Static Build Guide for Ratchet

This guide explains how to build fully static binaries for Ratchet.

## Current Status

The codebase has been prepared for static builds by:
1. ✅ Replacing OpenSSL with rustls (pure Rust TLS)
2. ✅ SQLite is bundled by default with sqlx (no external SQLite needed)
3. ⚠️  Compression libraries (bzip2, zstd) still link dynamically but are only used by the `zip` crate

## Dependencies Analysis

### Already Static-Friendly
- **TLS/SSL**: Using rustls instead of OpenSSL
- **SQLite**: Bundled with sqlx by default
- **File watching**: Platform syscalls (no dynamic libs)

### Potential Issues
- **ring**: Requires C compiler for musl builds
- **bzip2-sys**: Used by zip crate for compression
- **zstd-sys**: Compression library

## Building Static Binaries

### Linux (musl)

1. Install required tools:
```bash
# Ubuntu/Debian
sudo apt-get install musl-tools

# Fedora
sudo dnf install musl-gcc

# Arch
sudo pacman -S musl
```

2. Add the musl target:
```bash
rustup target add x86_64-unknown-linux-musl
```

3. Build:
```bash
cargo build --target x86_64-unknown-linux-musl --release
```

### macOS

Static linking is not fully supported on macOS. Use the standard build:
```bash
cargo build --release
```

### Windows

Windows builds are statically linked by default:
```bash
cargo build --release
```

## GitHub Actions

The manual build workflow already supports musl targets for static Linux builds. When triggered from the GitHub Actions UI, it will produce static binaries for Linux.

## Verification

To verify a binary is statically linked:

```bash
# Linux
ldd target/x86_64-unknown-linux-musl/release/ratchet
# Should output: "not a dynamic executable"

# Check file type
file target/x86_64-unknown-linux-musl/release/ratchet
# Should show: "statically linked"
```

## Future Improvements

1. Consider replacing the `zip` crate with a pure-Rust alternative to remove bzip2/zstd dependencies
2. Monitor ring crate updates for better musl support
3. Add static build verification to CI/CD pipeline