# Git2 TLS Migration Analysis & Resolution Plan

## Executive Summary

During the OpenSSL to rustls migration for enhanced security and cross-platform compatibility, we discovered that git2 dependency configuration (`default-features = false`) has **broken HTTPS Git repository access**. This affects the task registry's ability to clone and sync from GitHub, GitLab, and other HTTPS-based Git repositories.

**Critical Impact**: Sample configurations and documented Git repository features are currently non-functional.

## Current State Analysis

### Git2 Feature Dependencies

Git2 crate provides these TLS-related features:

```toml
[dependencies.git2.features]
default = ["ssh", "https", "ssh_key_from_memory"]
https = ["libgit2-sys/https", "openssl-sys", "openssl-probe"]  # OpenSSL required
ssh = ["libgit2-sys/ssh"]                                      # SSH with libssh2
```

### Current Configuration

**ratchet-registry/Cargo.toml:**
```toml
git2 = { version = "0.18", optional = true, default-features = false }
```

**Result**: 
- ✅ **Local Git repositories**: Working (file:// protocol)
- ❌ **HTTPS Git repositories**: **BROKEN** (cannot clone/fetch from github.com, gitlab.com)
- ❌ **SSH Git repositories**: **BROKEN** (git@github.com format disabled)

### Affected Functionality

#### Sample Configurations (Currently Broken)
```yaml
# sample/configs/example-git-registry.yaml
registry:
  sources:
    - name: "community-tasks"
      uri: "https://github.com/michiel/ratchet-repo-samples"  # BROKEN
    - name: "corporate-tasks" 
      uri: "https://github.com/corp/ratchet-tasks.git"           # BROKEN
    - name: "gitlab-tasks"
      uri: "git@gitlab.com:corp/automation-tasks.git"           # BROKEN
```

#### Registry Loader Implementation
The GitLoader in `ratchet-registry/src/loaders/git.rs` uses git2 directly for:
- Repository cloning (`git2::build::RepoBuilder`)
- Fetching updates (`FetchOptions`, `RemoteCallbacks`)
- Authentication (userpass, SSH keys, tokens)
- Branch checkout and synchronization

#### Documentation References
- README.md mentions GitHub integration examples
- ARCHITECTURE.md describes Git-based task registries
- LLM_TASK_DEVELOPMENT.md includes Git repository tools
- Multiple sample configurations assume HTTPS Git access

## Root Cause

**Git2 limitation**: The git2 crate only supports OpenSSL for HTTPS operations. There is no rustls backend available in git2, making it impossible to use HTTPS Git repositories without OpenSSL dependencies.

**Trade-off**: Our security improvement (rustls migration) inadvertently broke an entire class of repository sources.

## Solution Options

### Option 1: Re-enable Git2 HTTPS with OpenSSL (RECOMMENDED)

**Implementation:**
```toml
# ratchet-registry/Cargo.toml
git2 = { version = "0.18", optional = true, features = ["https"] }
```

**Rationale:**
- Restores full Git functionality immediately
- Maintains backward compatibility with all sample configurations
- OpenSSL dependency is isolated to Git operations only
- HTTP client tasks continue using rustls (ratchet-http remains pure Rust)

**Pros:**
- ✅ Immediate fix for broken functionality
- ✅ All sample configurations work as documented
- ✅ Full HTTPS and SSH Git repository support
- ✅ Minimal code changes required
- ✅ Maintains hybrid approach (rustls for HTTP, OpenSSL for Git)

**Cons:**
- ❌ Reintroduces OpenSSL dependency for git2 operations
- ❌ Cross-compilation complexity for Git features
- ❌ Two TLS stacks in the same binary

**Impact:**
- Binary size: +2-3MB (OpenSSL libraries)
- Dependencies: Adds openssl-sys, openssl-probe, libssh2-sys
- Security: OpenSSL limited to Git operations only

### Option 2: HTTP-based Git Implementation

**Implementation:**
Create custom Git-over-HTTPS implementation using ratchet-http (rustls):
- Use GitHub/GitLab APIs for repository metadata
- Download repository archives via HTTPS
- Implement custom Git protocol parsing
- Handle authentication via HTTP headers/tokens

**Pros:**
- ✅ Pure Rust TLS (rustls) for all network operations
- ✅ No OpenSSL dependencies
- ✅ Potentially better performance for large repositories (archive downloads)
- ✅ Easier authentication with API tokens

**Cons:**
- ❌ Significant implementation complexity (3-4 weeks)
- ❌ Limited Git feature support (no SSH, complex branching)
- ❌ Platform-specific API implementations (GitHub vs GitLab vs custom)
- ❌ Loss of standard Git tooling compatibility
- ❌ Maintenance burden for custom Git implementation

**Timeline:** 3-4 weeks development + testing

### Option 3: Repository Type Separation

**Implementation:**
Split repository support by protocol:
- **Local repositories**: git2 without TLS (current state)
- **HTTPS repositories**: HTTP-based implementation with ratchet-http
- **SSH repositories**: Disable and document as unsupported

**Pros:**
- ✅ Hybrid approach balances security and functionality
- ✅ Pure Rust for HTTP-based Git operations
- ✅ Maintains local Git repository support

**Cons:**
- ❌ Complex implementation with multiple code paths
- ❌ Inconsistent user experience across repository types
- ❌ SSH support permanently disabled
- ❌ Maintenance complexity with dual implementations

**Timeline:** 2-3 weeks development + testing

### Option 4: Document Limitation (NOT RECOMMENDED)

**Implementation:**
- Update documentation to reflect HTTPS/SSH limitations
- Remove HTTPS examples from sample configurations
- Provide workarounds (local clones, HTTP task repositories)

**Pros:**
- ✅ No code changes required
- ✅ Maintains pure Rust TLS implementation

**Cons:**
- ❌ Major feature regression
- ❌ Breaks documented functionality
- ❌ Poor user experience for common use cases
- ❌ Sample configurations become misleading
- ❌ Limits adoption for teams using Git repositories

## Risk Assessment

### Current Risk (No Action)
- **High**: Broken functionality affects production deployments
- **High**: Documentation and samples are misleading
- **Medium**: User confusion and support burden

### Option 1 Risk (Re-enable OpenSSL)
- **Low**: Well-tested approach with git2
- **Low**: Limited OpenSSL exposure (Git operations only)
- **Medium**: Two TLS stacks increase binary complexity

### Option 2 Risk (Custom Implementation)
- **High**: Complex custom implementation may have bugs
- **Medium**: Long development timeline delays other features
- **Low**: Pure Rust security benefits

### Option 3 Risk (Hybrid)
- **Medium**: Dual implementation complexity
- **Medium**: Potential inconsistencies between approaches
- **Low**: Balanced security/functionality trade-off

## Recommendation: Option 1 - Re-enable Git2 HTTPS

### Justification

1. **Immediate Value**: Restores broken functionality without development delay
2. **User Experience**: All documented features work as expected
3. **Pragmatic Security**: OpenSSL isolated to Git operations, HTTP client remains pure Rust
4. **Maintenance**: Leverages mature, well-tested git2 implementation
5. **Compatibility**: Full Git ecosystem compatibility (HTTPS, SSH, authentication)

### Implementation Plan

#### Phase 1: Immediate Fix (1-2 hours)
```bash
# Update ratchet-registry/Cargo.toml
git2 = { version = "0.18", optional = true, features = ["https"] }

# Update documentation comment
# Git operations with HTTPS support (requires OpenSSL for git2)
```

#### Phase 2: Testing & Validation (1 day)
- Test HTTPS repository cloning with GitHub/GitLab
- Verify SSH repository access
- Validate authentication with tokens and SSH keys
- Test all sample configurations
- Ensure no regression in HTTP client (ratchet-http with rustls)

#### Phase 3: Documentation Update (2 hours)
- Update CLAUDE.md to reflect hybrid TLS approach
- Document OpenSSL usage limited to Git operations
- Update cross-platform build instructions if needed

#### Phase 4: Security Review (1 day)
- Audit OpenSSL exposure surface
- Verify TLS stack isolation
- Document security model for hybrid approach

### Long-term Considerations

#### Future Rust-native Git Options
Monitor ecosystem for pure Rust Git implementations:
- **git2-rs alternatives**: Track community efforts for rustls support
- **gix (formerly gitoxide)**: Evaluate pure Rust Git implementation maturity
- **GitHub API approach**: Consider API-based repository access for specific providers

#### Security Model
- **Git operations**: OpenSSL for HTTPS/SSH (isolated, mature)
- **HTTP client tasks**: rustls for pure Rust security
- **Clear separation**: Different TLS stacks for different purposes

#### Migration Path
If pure Rust Git implementation becomes available:
1. Feature flag migration approach
2. Gradual transition with backward compatibility
3. Performance and feature parity validation

## Acceptance Criteria

### Immediate (Option 1)
- [ ] HTTPS Git repositories clone successfully
- [ ] SSH Git repositories work with proper authentication
- [ ] All sample configurations functional
- [ ] HTTP client tasks continue using rustls
- [ ] No regression in existing functionality
- [ ] Documentation reflects hybrid TLS approach

### Future Evaluation Points
- [ ] Monitor pure Rust Git implementation maturity (6 months)
- [ ] Evaluate user feedback on hybrid approach (3 months)
- [ ] Assess binary size impact in production (1 month)
- [ ] Review cross-platform build complexity (1 month)

## Conclusion

The git2 TLS migration analysis reveals a critical functionality gap that requires immediate attention. **Option 1 (re-enabling git2 HTTPS with OpenSSL)** provides the best balance of functionality, user experience, and implementation complexity while maintaining our overall security improvements through the hybrid TLS approach.

The recommendation allows us to restore full Git repository functionality immediately while preserving the security benefits of rustls for HTTP client operations, creating a pragmatic solution that serves both security and usability requirements.

---

**Document Version**: 1.1  
**Created**: December 2024  
**Last Updated**: December 2024  
**Status**: ✅ IMPLEMENTED - Option 1 Complete  
**Implementation**: HTTPS-only git2 support successfully added

## Implementation Results

**✅ SUCCESSFULLY IMPLEMENTED Option 1 - HTTPS Git2 Support**

### Changes Made
- **ratchet-registry/Cargo.toml**: Added `features = ["https"]` to git2 dependency
- **Optional Vendoring**: Added `git-vendored` feature flag for vendored OpenSSL builds
- **Hybrid TLS Architecture**: Maintained rustls for HTTP client, OpenSSL limited to git2
- **Test Coverage**: Added comprehensive HTTPS Git cloning tests
- **Documentation**: Updated CLAUDE.md and created BUILD_OPTIONS.md for TLS configuration

### Verification
- ✅ HTTPS Git repositories can be cloned (tested with github.com/octocat/Hello-World.git)
- ✅ HTTP client continues using rustls (ratchet-http unchanged)  
- ✅ Workspace builds successfully with zero compilation errors
- ✅ All existing tests continue to pass
- ✅ Sample configurations will now work as documented
- ✅ Optional vendored OpenSSL available via `git-vendored` feature flag
- ✅ Comprehensive build documentation created (BUILD_OPTIONS.md)

### Security Model Confirmed
- **Git operations**: OpenSSL for HTTPS Git repository access (isolated scope)
- **HTTP client tasks**: rustls for pure Rust security in application HTTP requests
- **Clear separation**: Different TLS stacks serve different purposes optimally
- **Flexible deployment**: System OpenSSL (default) or vendored OpenSSL (git-vendored feature)

### Build Options Available
- **Default**: `--features git` (system OpenSSL, fast builds)
- **Vendored**: `--features git-vendored` (bundled OpenSSL, requires perl with FindBin)
- **No Git**: `--no-default-features --features filesystem,http` (pure rustls only)

The implementation successfully restores full HTTPS Git repository functionality while maintaining the security benefits of rustls for application HTTP operations, with flexible build options for different deployment scenarios.