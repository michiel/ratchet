# Cross-Platform Considerations for Ratchet Output

This document outlines the cross-platform compatibility features and considerations for the `ratchet-output` crate.

## Platform Support

The `ratchet-output` crate is designed to work seamlessly across:
- **Linux** (all major distributions)
- **macOS** (10.14+)
- **Windows** (Windows 10+)

## Filesystem Destination

### Path Handling

#### Automatic Path Normalization
- Unix-style paths (`/results/2024/01/output.json`) are automatically converted to platform-specific separators
- Windows paths use backslashes (`\results\2024\01\output.json`)
- Path templates support both forward slashes and platform-specific separators

#### Windows-Specific Validations
- **Invalid Characters**: Automatically rejects paths containing `< > : " | ? *`
- **Reserved Names**: Prevents use of Windows reserved filenames (CON, PRN, AUX, NUL, COM1-9, LPT1-9)
- **Length Limits**: Validates against Windows path length restrictions

#### Universal Validations
- **Null Bytes**: Rejected on all platforms (invalid everywhere)
- **Path Structure**: Ensures valid directory hierarchies

### File Permissions

#### Unix/Linux/macOS
```yaml
filesystem:
  path: "/results/{{job_id}}.json"
  permissions: 0o644  # Standard: owner read/write, group/other read
  create_dirs: true
```

Common permission patterns:
- `0o644` - Owner read/write, others read
- `0o600` - Owner read/write only
- `0o755` - Executable directories

#### Windows
```yaml
filesystem:
  path: "C:\\Results\\{{job_id}}.json"
  permissions: 644  # Converted to read-only flag if needed
  create_dirs: true
```

On Windows:
- Permissions are simplified to read-only flag
- ACLs (Access Control Lists) are handled by the OS
- Directory creation uses inherited permissions

### Example Configurations

#### Cross-Platform Safe Path Template
```yaml
output_destinations:
  - type: filesystem
    path: "results/{{env}}/{{date}}/{{job_id}}.json"  # Works on all platforms
    format: json
    permissions: 644
    create_dirs: true
    overwrite: false
```

#### Platform-Specific Paths
```yaml
# Unix/Linux/macOS
- type: filesystem
  path: "/var/log/ratchet/{{job_id}}.json"
  
# Windows
- type: filesystem
  path: "C:\\ProgramData\\Ratchet\\Logs\\{{job_id}}.json"
```

## HTTP/Webhook Destination

### TLS Configuration
The webhook destination uses `rustls-tls` for cross-platform TLS support:
- **No OpenSSL dependency** - pure Rust implementation
- **Consistent behavior** across all platforms
- **Built-in certificate validation** using webpki

### Default HTTP Client Settings
```rust
reqwest::Client::builder()
    .use_rustls_tls()                           // Cross-platform TLS
    .connect_timeout(Duration::from_secs(10))   // Connection timeout
    .pool_idle_timeout(Duration::from_secs(30)) // Connection pooling
    .user_agent("ratchet-output/1.0")          // Consistent user agent
    .build()
```

### Network Considerations
- **IPv6 Support**: Automatic dual-stack support
- **Proxy Detection**: Respects system proxy settings
- **DNS Resolution**: Platform-native resolution

## Template Engine

### Path Separators in Templates
```handlebars
<!-- Cross-platform safe (automatically converted) -->
{{base_path}}/{{year}}/{{month}}/{{filename}}

<!-- Platform-specific if needed -->
{{#if windows}}
{{base_path}}\{{year}}\{{month}}\{{filename}}
{{else}}
{{base_path}}/{{year}}/{{month}}/{{filename}}
{{/if}}
```

### Common Template Variables
```yaml
template_variables:
  base_path: "/var/log/ratchet"     # Unix
  base_path: "C:\\Logs\\Ratchet"   # Windows
  date: "2024-01-06"
  job_id: "12345"
  env: "production"
```

## Error Handling

### Platform-Specific Errors
The crate provides detailed error information for platform-specific issues:

```rust
match delivery_result {
    Err(DeliveryError::CrossPlatformPath { path, error }) => {
        // Handle path-specific issues
        eprintln!("Invalid path for this platform: {} - {}", path, error);
    }
    Err(DeliveryError::Filesystem { path, operation, error }) => {
        // Handle filesystem operations
        eprintln!("Filesystem error at {}: {}", path, error);
    }
    _ => {}
}
```

### Common Error Scenarios

#### Windows
- **Path too long** (>260 characters without long path support)
- **Invalid characters** in filenames
- **Permission denied** due to file locks or ACLs
- **Reserved filenames** (CON.txt, PRN.json, etc.)

#### Unix/Linux/macOS
- **Permission denied** due to insufficient file permissions
- **No space left** on device
- **Read-only filesystem**
- **Invalid symlinks**

## Testing

### Cross-Platform Test Suite
The crate includes comprehensive tests that validate:
- Path normalization across platforms
- Permission handling differences
- Error message consistency
- Template rendering with platform-specific paths

### Running Tests
```bash
# Run all tests
cargo test

# Run platform-specific tests
cargo test test_path_validation_windows
cargo test test_cross_platform_path_normalization
```

## Best Practices

### 1. Use Relative Paths When Possible
```yaml
# Good - works everywhere
path: "results/{{job_id}}.json"

# Avoid - platform-specific
path: "/absolute/unix/path.json"
path: "C:\\absolute\\windows\\path.json"
```

### 2. Validate Configurations Early
```rust
// Test configurations before deployment
let test_results = OutputDeliveryManager::test_configurations(&configs).await?;
for result in test_results {
    if !result.success {
        eprintln!("Config {} failed: {}", result.index, result.error.unwrap());
    }
}
```

### 3. Handle Platform Differences Gracefully
```rust
let permissions = if cfg!(windows) {
    0o644  // Will be converted to read-only flag
} else {
    0o644  // Full Unix permissions
};
```

### 4. Use Standard Directory Locations
```yaml
# Prefer standard locations
base_paths:
  unix: "/var/log/ratchet"
  windows: "%PROGRAMDATA%\\Ratchet\\Logs"
  user_unix: "$HOME/.local/share/ratchet"
  user_windows: "%APPDATA%\\Ratchet"
```

## Migration Guide

If you're migrating from platform-specific code:

### Before (Platform-Specific)
```yaml
# Unix-only configuration
output_destinations:
  - type: filesystem
    path: "/var/log/ratchet/{{job_id}}.json"
    permissions: 0o644
```

### After (Cross-Platform)
```yaml
# Cross-platform configuration
output_destinations:
  - type: filesystem
    path: "{{log_dir}}/ratchet/{{job_id}}.json"
    permissions: 644  # Interpreted per platform
    create_dirs: true
    
template_variables:
  log_dir: "/var/log"      # Set per deployment
```

## Performance Considerations

### File System Performance
- **Windows**: NTFS provides good performance for large directories
- **Linux**: ext4/XFS handle millions of files efficiently
- **macOS**: APFS optimized for modern workloads

### Network Performance
- **Connection Pooling**: Automatically enabled for webhooks
- **DNS Caching**: Built into reqwest client
- **Keep-Alive**: Maintained across requests

## Security Considerations

### File Permissions
- **Unix**: Respect umask and file permissions
- **Windows**: Integrate with NTFS permissions and ACLs
- **Principle of Least Privilege**: Set minimal required permissions

### Network Security
- **TLS 1.2+**: Enforced by rustls
- **Certificate Validation**: Automatic with webpki
- **No Insecure Fallbacks**: HTTPS-only by default

## Troubleshooting

### Common Issues

#### "Invalid path characters" on Windows
```
Error: Cross-platform path error: results/output<test>.json - Path contains invalid characters for Windows
```
**Solution**: Remove `< > : " | ? *` characters from path templates

#### "Permission denied" on Unix
```
Error: Filesystem operation failed at /results/output.json (write): Permission denied
```
**Solution**: Check directory permissions and ownership

#### "Reserved filename" on Windows
```
Error: Cross-platform path error: CON.json - Filename 'CON.json' is reserved on Windows
```
**Solution**: Avoid Windows reserved names (CON, PRN, AUX, etc.)

### Debug Mode
Enable debug logging to see cross-platform path handling:

```rust
RUST_LOG=ratchet_output=debug cargo run
```

This will show:
- Path normalization steps
- Permission conversions
- Platform-specific validations
- HTTP client configuration