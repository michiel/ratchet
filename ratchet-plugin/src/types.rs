//! Plugin type definitions and utilities

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

/// Plugin type enumeration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PluginType {
    /// Task execution plugin
    Task,
    /// Output destination plugin
    Output,
    /// Authentication plugin
    Auth,
    /// Logging plugin
    Logging,
    /// Monitoring plugin
    Monitoring,
    /// Cache plugin
    Cache,
    /// Registry plugin
    Registry,
    /// Custom plugin type
    Custom(String),
}

impl fmt::Display for PluginType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Task => write!(f, "task"),
            Self::Output => write!(f, "output"),
            Self::Auth => write!(f, "auth"),
            Self::Logging => write!(f, "logging"),
            Self::Monitoring => write!(f, "monitoring"),
            Self::Cache => write!(f, "cache"),
            Self::Registry => write!(f, "registry"),
            Self::Custom(name) => write!(f, "{}", name),
        }
    }
}

/// Plugin version with semantic versioning
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginVersion {
    /// Semantic version
    pub version: semver::Version,
    /// Pre-release identifier
    pub pre_release: Option<String>,
    /// Build metadata
    pub build: Option<String>,
}

impl PluginVersion {
    /// Create a new plugin version
    pub fn new(major: u64, minor: u64, patch: u64) -> Self {
        Self {
            version: semver::Version::new(major, minor, patch),
            pre_release: None,
            build: None,
        }
    }

    /// Create from string
    pub fn parse(s: &str) -> Result<Self, semver::Error> {
        let version = semver::Version::parse(s)?;
        Ok(Self {
            version,
            pre_release: None,
            build: None,
        })
    }

    /// Check if this version is compatible with a requirement
    pub fn is_compatible(&self, requirement: &VersionRequirement) -> bool {
        requirement.matches(&self.version)
    }
}

impl fmt::Display for PluginVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.version)?;
        if let Some(ref pre) = self.pre_release {
            write!(f, "-{}", pre)?;
        }
        if let Some(ref build) = self.build {
            write!(f, "+{}", build)?;
        }
        Ok(())
    }
}

/// Version requirement for plugin dependencies
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VersionRequirement {
    /// Version requirement string (e.g., "^1.0.0", ">=2.0.0")
    pub requirement: String,
    #[serde(skip)]
    parsed: Option<semver::VersionReq>,
}

impl VersionRequirement {
    /// Create a new version requirement
    pub fn new(requirement: impl Into<String>) -> Result<Self, semver::Error> {
        let requirement = requirement.into();
        let parsed = semver::VersionReq::parse(&requirement)?;
        Ok(Self {
            requirement,
            parsed: Some(parsed),
        })
    }

    /// Check if a version matches this requirement
    pub fn matches(&self, version: &semver::Version) -> bool {
        if let Some(ref parsed) = self.parsed {
            parsed.matches(version)
        } else {
            // Try to parse on demand if not already parsed
            semver::VersionReq::parse(&self.requirement)
                .map(|req| req.matches(version))
                .unwrap_or(false)
        }
    }
}

impl fmt::Display for VersionRequirement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.requirement)
    }
}

/// Plugin dependency specification
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginDependency {
    /// Dependency name
    pub name: String,
    /// Version requirement
    pub version: VersionRequirement,
    /// Whether this dependency is optional
    #[serde(default)]
    pub optional: bool,
    /// Additional metadata
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

impl PluginDependency {
    /// Create a new plugin dependency
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Result<Self, semver::Error> {
        Ok(Self {
            name: name.into(),
            version: VersionRequirement::new(version.into())?,
            optional: false,
            metadata: HashMap::new(),
        })
    }

    /// Make this dependency optional
    pub fn optional(mut self) -> Self {
        self.optional = true;
        self
    }

    /// Add metadata to this dependency
    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }
}

/// Plugin capability flags
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginCapabilities {
    /// Plugin can handle async operations
    #[serde(default)]
    pub async_support: bool,
    /// Plugin supports hot reloading
    #[serde(default)]
    pub hot_reload: bool,
    /// Plugin has configuration
    #[serde(default)]
    pub configurable: bool,
    /// Plugin requires elevated permissions
    #[serde(default)]
    pub requires_permissions: bool,
    /// Plugin supports multiple instances
    #[serde(default)]
    pub multi_instance: bool,
    /// Custom capabilities
    #[serde(default)]
    pub custom: HashMap<String, bool>,
}

impl Default for PluginCapabilities {
    fn default() -> Self {
        Self {
            async_support: true,
            hot_reload: false,
            configurable: false,
            requires_permissions: false,
            multi_instance: false,
            custom: HashMap::new(),
        }
    }
}

impl PluginCapabilities {
    /// Check if a capability is supported
    pub fn supports(&self, capability: &str) -> bool {
        match capability {
            "async" => self.async_support,
            "hot_reload" => self.hot_reload,
            "configurable" => self.configurable,
            "permissions" => self.requires_permissions,
            "multi_instance" => self.multi_instance,
            _ => self.custom.get(capability).copied().unwrap_or(false),
        }
    }

    /// Add a custom capability
    pub fn with_custom_capability(mut self, name: impl Into<String>, supported: bool) -> Self {
        self.custom.insert(name.into(), supported);
        self
    }
}

/// Plugin status enumeration
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PluginStatus {
    /// Plugin is not loaded
    Unloaded,
    /// Plugin is loading
    Loading,
    /// Plugin is loaded and active
    Active,
    /// Plugin is disabled
    Disabled,
    /// Plugin failed to load or execute
    Failed,
    /// Plugin is being unloaded
    Unloading,
}

impl fmt::Display for PluginStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unloaded => write!(f, "unloaded"),
            Self::Loading => write!(f, "loading"),
            Self::Active => write!(f, "active"),
            Self::Disabled => write!(f, "disabled"),
            Self::Failed => write!(f, "failed"),
            Self::Unloading => write!(f, "unloading"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_version() {
        let version = PluginVersion::new(1, 2, 3);
        assert_eq!(version.to_string(), "1.2.3");

        let parsed = PluginVersion::parse("2.0.0-beta.1").unwrap();
        assert_eq!(parsed.version.major, 2);
        assert_eq!(parsed.version.minor, 0);
        assert_eq!(parsed.version.patch, 0);
    }

    #[test]
    fn test_version_requirement() {
        let req = VersionRequirement::new("^1.0.0").unwrap();
        let version = semver::Version::new(1, 2, 3);
        assert!(req.matches(&version));

        let version2 = semver::Version::new(2, 0, 0);
        assert!(!req.matches(&version2));
    }

    #[test]
    fn test_plugin_dependency() {
        let dep = PluginDependency::new("test-plugin", "^1.0.0").unwrap();
        assert_eq!(dep.name, "test-plugin");
        assert!(!dep.optional);

        let optional_dep = dep.optional();
        assert!(optional_dep.optional);
    }

    #[test]
    fn test_plugin_capabilities() {
        let mut caps = PluginCapabilities::default();
        assert!(caps.supports("async"));
        assert!(!caps.supports("hot_reload"));

        caps = caps.with_custom_capability("custom_feature", true);
        assert!(caps.supports("custom_feature"));
    }

    #[test]
    fn test_plugin_type_display() {
        assert_eq!(PluginType::Task.to_string(), "task");
        assert_eq!(PluginType::Custom("mytype".to_string()).to_string(), "mytype");
    }

    #[test]
    fn test_plugin_status_display() {
        assert_eq!(PluginStatus::Active.to_string(), "active");
        assert_eq!(PluginStatus::Failed.to_string(), "failed");
    }
}