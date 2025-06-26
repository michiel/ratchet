# Linux Package Distribution Implementation Plan

**Date:** 2025-06-24  
**Status:** Planning  
**Priority:** Medium  
**Scope:** Add Debian (.deb) and RPM (.rpm) package support to release workflow

## Executive Summary

This document outlines the requirements and implementation strategy for adding professional Linux package distribution capabilities to the Ratchet project. The current release workflow produces static binaries; this plan extends it to generate native Linux packages for mainstream distributions.

## Current State Analysis

### Existing Release Workflow
- **File:** `.github/workflows/release.yml`
- **Platforms:** Linux (x86_64/aarch64), Windows, macOS
- **Output:** Static musl binaries via `houseabsolute/actions-rust-cross`
- **Distribution:** GitHub Releases with tar.gz archives

### Limitations
- No native Linux package manager integration
- Manual installation process for users
- No systemd service management
- No automatic dependency resolution
- Missing standard Linux filesystem layout

## Requirements Analysis

### 1. Debian Package (.deb) Requirements

#### Package Structure
```
debian/
├── control           # Package metadata, dependencies, conflicts
├── changelog         # Debian changelog format (dch tool)
├── copyright         # License and copyright information
├── rules             # Build rules (makefile-like, executable)
├── compat            # Debhelper compatibility level (13+)
├── install           # File installation mapping
├── postinst          # Post-installation script (systemd enable)
├── prerm             # Pre-removal script (systemd disable)
├── postrm            # Post-removal script (cleanup)
└── ratchet.service   # systemd service definition
```

#### Key Metadata
- **Package Name:** `ratchet`
- **Architecture:** `amd64`, `arm64`
- **Section:** `utils` or `admin`
- **Priority:** `optional`
- **Dependencies:** `libc6`, `libssl3`, `ca-certificates`
- **Maintainer:** Ratchet Project team
- **Description:** Task automation and execution platform

#### Build Dependencies
- `debhelper (>= 13)`
- `dpkg-dev`
- `fakeroot`
- `lintian` (package validation)

### 2. RPM Package (.rpm) Requirements

#### Specification File Structure
```
ratchet.spec
├── %description      # Package description
├── %prep             # Preparation phase
├── %build            # Build phase (usually empty for pre-built)
├── %install          # Installation phase
├── %files            # File list with permissions
├── %post             # Post-installation scriptlet
├── %preun            # Pre-uninstallation scriptlet
└── %postun           # Post-uninstallation scriptlet
```

#### Key Metadata
- **Name:** `ratchet`
- **Version:** `${VERSION}` (from release tag)
- **Release:** `1%{?dist}`
- **License:** `MIT OR Apache-2.0`
- **Group:** `Applications/System`
- **BuildArch:** `x86_64`, `aarch64`
- **Requires:** `glibc`, `openssl-libs`, `ca-certificates`

#### Build Dependencies
- `rpm-build`
- `rpmlint`
- `rpmdevtools`

### 3. System Integration Requirements

#### Systemd Service File
```systemd
[Unit]
Description=Ratchet Task Automation Platform
Documentation=https://github.com/ratchet-runner/ratchet
After=network.target
Wants=network.target

[Service]
Type=exec
ExecStart=/usr/bin/ratchet server
ExecReload=/bin/kill -HUP $MAINPID
Restart=on-failure
RestartSec=10
User=ratchet
Group=ratchet
WorkingDirectory=/var/lib/ratchet
Environment=RATCHET_CONFIG_DIR=/etc/ratchet
Environment=RATCHET_DATA_DIR=/var/lib/ratchet
Environment=RATCHET_LOG_DIR=/var/log/ratchet

# Security settings
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ReadWritePaths=/var/lib/ratchet /var/log/ratchet
ProtectHome=true
CapabilityBoundingSet=

[Install]
WantedBy=multi-user.target
```

#### Filesystem Layout
```
/usr/bin/ratchet                    # Main executable
/etc/ratchet/
├── config.yaml                     # Default configuration
└── config.d/                       # Configuration fragments
/etc/default/ratchet                 # Environment variables
/var/lib/ratchet/                    # Application data
├── data/                            # Database and state
└── tasks/                           # Task definitions
/var/log/ratchet/                    # Log files
/lib/systemd/system/ratchet.service  # systemd service
/usr/share/doc/ratchet/              # Documentation
├── README.md
├── CHANGES.md
├── CLI_USAGE.md
└── LLM_TASK_DEVELOPMENT.md
```

#### User and Group Management
```bash
# Pre-installation
getent group ratchet >/dev/null || groupadd -r ratchet
getent passwd ratchet >/dev/null || \
    useradd -r -g ratchet -d /var/lib/ratchet -s /sbin/nologin \
    -c "Ratchet service user" ratchet
```

## Implementation Strategy

### Phase 1: Infrastructure Setup

#### 1.1 Create Package Metadata Files
```bash
mkdir -p debian/
mkdir -p packaging/rpm/
mkdir -p packaging/systemd/
mkdir -p packaging/config/
```

#### 1.2 Debian Package Files
- `debian/control` - Package dependencies and metadata
- `debian/changelog` - Version history in Debian format
- `debian/rules` - Build automation (debhelper)
- `debian/install` - File installation mapping
- `debian/postinst` - User creation, systemd enable
- `debian/prerm` - systemd disable and stop

#### 1.3 RPM Specification
- `packaging/rpm/ratchet.spec` - Complete RPM spec file
- Scripts for user management and systemd integration

#### 1.4 Supporting Files
- `packaging/systemd/ratchet.service` - systemd service definition
- `packaging/config/config.yaml` - Default configuration template
- `packaging/scripts/` - Installation/removal scripts

### Phase 2: Tooling Integration

#### 2.1 nFPM Configuration (Recommended)
Create `nfpm.yaml` for unified package building:

```yaml
name: "ratchet"
arch: "${ARCH}"
platform: "linux"
version: "${VERSION}"
version_schema: "semver"
maintainer: "Ratchet Project <noreply@ratchet.dev>"
description: |
  Task automation and execution platform with GraphQL, REST, and MCP APIs.
  Provides workflow orchestration, task scheduling, and plugin management.
vendor: "Ratchet Project"
homepage: "https://github.com/ratchet-runner/ratchet"
license: "MIT OR Apache-2.0"
section: "utils"
priority: "optional"

depends:
  - libc6
  - libssl3 | libssl1.1
  - ca-certificates

provides:
  - ratchet

contents:
  # Main executable
  - src: "./target/${TARGET}/release/ratchet"
    dst: "/usr/bin/ratchet"
    file_info:
      mode: 0755
  
  # Systemd service
  - src: "./packaging/systemd/ratchet.service"
    dst: "/lib/systemd/system/ratchet.service"
    file_info:
      mode: 0644
  
  # Configuration
  - src: "./packaging/config/config.yaml"
    dst: "/etc/ratchet/config.yaml"
    type: config
    file_info:
      mode: 0644
  
  # Documentation
  - src: "./docs/CLI_USAGE.md"
    dst: "/usr/share/doc/ratchet/CLI_USAGE.md"
  - src: "./docs/LLM_TASK_DEVELOPMENT.md"
    dst: "/usr/share/doc/ratchet/LLM_TASK_DEVELOPMENT.md"
  - src: "./CHANGES.md"
    dst: "/usr/share/doc/ratchet/CHANGES.md"
  
  # Directories
  - dst: "/var/lib/ratchet"
    type: dir
    file_info:
      mode: 0755
      owner: ratchet
      group: ratchet
  - dst: "/var/log/ratchet"
    type: dir
    file_info:
      mode: 0755
      owner: ratchet
      group: ratchet
  - dst: "/etc/ratchet/config.d"
    type: dir
    file_info:
      mode: 0755

scripts:
  preinstall: "./packaging/scripts/preinstall.sh"
  postinstall: "./packaging/scripts/postinstall.sh"
  preremove: "./packaging/scripts/preremove.sh"
  postremove: "./packaging/scripts/postremove.sh"

deb:
  lintian_overrides:
    - "ratchet: binary-without-manpage usr/bin/ratchet"

rpm:
  group: "Applications/System"
  compression: "xz"
```

#### 2.2 Alternative: Platform-Specific Tools
- **Debian:** `jiro4989/build-deb-action@v3`
- **RPM:** `bpetersen/build-rpm-action@v1`
- **Docker-based:** Custom containers with packaging tools

### Phase 3: GitHub Actions Integration

#### 3.1 Extended Release Workflow
```yaml
# Add to existing .github/workflows/release.yml
package-linux:
  name: Build Linux Packages
  needs: release
  runs-on: ubuntu-latest
  if: github.event_name == 'release'
  
  strategy:
    matrix:
      arch: [x86_64, aarch64]
      format: [deb, rpm]
  
  steps:
    - name: Checkout
      uses: actions/checkout@v4
      
    - name: Download binary artifacts
      uses: actions/download-artifact@v4
      with:
        name: ratchet-${{ github.event.release.tag_name }}-linux-${{ matrix.arch }}
        path: ./artifacts/
    
    - name: Install nFPM
      run: |
        curl -sfL https://install.goreleaser.com/github.com/goreleaser/nfpm.sh | sh
        sudo mv ./bin/nfpm /usr/local/bin/
    
    - name: Prepare package environment
      run: |
        mkdir -p target/linux-${{ matrix.arch }}/release/
        cp artifacts/ratchet target/linux-${{ matrix.arch }}/release/
        chmod +x target/linux-${{ matrix.arch }}/release/ratchet
    
    - name: Build ${{ matrix.format }} package
      env:
        VERSION: ${{ github.event.release.tag_name }}
        ARCH: ${{ matrix.arch == 'x86_64' && 'amd64' || 'arm64' }}
        TARGET: linux-${{ matrix.arch }}
      run: |
        nfpm package \
          --config nfpm.yaml \
          --packager ${{ matrix.format }} \
          --target ratchet-${VERSION}-linux-${{ matrix.arch }}.${{ matrix.format }}
    
    - name: Upload package to release
      uses: actions/upload-release-asset@v1
      env:
        GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
      with:
        upload_url: ${{ github.event.release.upload_url }}
        asset_path: ./ratchet-${{ github.event.release.tag_name }}-linux-${{ matrix.arch }}.${{ matrix.format }}
        asset_name: ratchet-${{ github.event.release.tag_name }}-linux-${{ matrix.arch }}.${{ matrix.format }}
        asset_content_type: application/octet-stream
```

#### 3.2 Package Validation
```yaml
- name: Validate packages
  run: |
    if [ "${{ matrix.format }}" = "deb" ]; then
      sudo dpkg -i ratchet-*.deb
      dpkg -l | grep ratchet
      sudo systemctl status ratchet --no-pager
      sudo dpkg -r ratchet
    else
      sudo rpm -ivh ratchet-*.rpm
      rpm -qa | grep ratchet
      sudo systemctl status ratchet --no-pager
      sudo rpm -e ratchet
    fi
```

### Phase 4: Repository Distribution (Future)

#### 4.1 APT Repository Setup
- GitHub Pages hosting for Debian repository
- GPG signing for package authenticity
- `Release`, `Packages.gz` metadata generation

#### 4.2 YUM/DNF Repository Setup
- RPM repository metadata (`repodata/`)
- GPG signing and key distribution
- Repository configuration files

## Testing Strategy

### Package Installation Testing
```bash
# Docker-based testing
docker run --rm -v $(pwd):/workspace ubuntu:22.04 bash -c "
  cd /workspace && 
  apt update && 
  apt install -y ./ratchet-*.deb && 
  systemctl status ratchet
"

docker run --rm -v $(pwd):/workspace fedora:38 bash -c "
  cd /workspace && 
  dnf install -y ./ratchet-*.rpm && 
  systemctl status ratchet
"
```

### Automated Testing Matrix
- **Distributions:** Ubuntu 20.04/22.04/24.04, Debian 11/12, RHEL 8/9, Fedora 38/39
- **Architectures:** x86_64, aarch64
- **Test Cases:** Install, start service, basic functionality, upgrade, remove

## Migration Path

### Immediate (Phase 1)
1. Create packaging metadata files
2. Add nFPM configuration
3. Create installation scripts

### Short-term (Phase 2)
1. Integrate package building into release workflow
2. Add package validation testing
3. Update documentation

### Long-term (Phase 3)
1. Set up APT/YUM repositories
2. Implement package signing
3. Add automatic updates mechanism

## Benefits

### For Users
- **Native Installation:** `apt install ratchet` or `dnf install ratchet`
- **Service Management:** `systemctl start/stop/status ratchet`
- **Automatic Updates:** Through package manager
- **Dependency Resolution:** Automatic handling of system dependencies
- **Standard Layout:** Follows Linux filesystem hierarchy

### For Project
- **Professional Distribution:** Industry-standard package formats
- **Broader Adoption:** Lower barrier to entry for Linux users
- **System Integration:** Native systemd and configuration management
- **Repository Metrics:** Package download and usage analytics

## Resource Requirements

### Development Time
- **Initial Setup:** 2-3 days
- **Testing and Validation:** 1-2 days
- **Documentation:** 1 day

### Maintenance Overhead
- **Per Release:** Additional 10-15 minutes build time
- **Repository Management:** 1-2 hours monthly (if implemented)

### Infrastructure
- **GitHub Actions:** Additional 5-10 minutes per release
- **Storage:** ~50MB additional artifacts per release
- **Optional:** Repository hosting costs (if external)

## Conclusion

Adding Debian and RPM package support would significantly improve the professional deployment experience for Ratchet on Linux systems. The implementation can be phased to minimize initial complexity while providing immediate value through native package installation and systemd integration.

The recommended approach using nFPM provides a unified solution for both package formats while maintaining flexibility for future enhancements such as repository distribution and automatic updates.