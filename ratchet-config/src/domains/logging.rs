//! Logging configuration

use serde::{Deserialize, Serialize};
use std::str::FromStr;
use crate::validation::{Validatable, validate_enum_choice, validate_required_string};
use crate::error::ConfigResult;

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LoggingConfig {
    /// Log level
    #[serde(default)]
    pub level: LogLevel,
    
    /// Log format
    #[serde(default)]
    pub format: LogFormat,
    
    /// Log targets configuration
    #[serde(default)]
    pub targets: Vec<LogTarget>,
    
    /// Whether to include source location in logs
    #[serde(default = "crate::domains::utils::default_false")]
    pub include_location: bool,
    
    /// Whether to enable structured logging
    #[serde(default = "crate::domains::utils::default_true")]
    pub structured: bool,
}

/// Log level enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

/// Log format enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogFormat {
    Json,
    Text,
    Compact,
    Pretty,
}

/// Log target configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum LogTarget {
    Console {
        #[serde(default)]
        level: Option<LogLevel>,
    },
    File {
        path: String,
        #[serde(default)]
        level: Option<LogLevel>,
        #[serde(default = "default_max_file_size")]
        max_size_bytes: usize,
        #[serde(default = "default_max_files")]
        max_files: usize,
    },
    Syslog {
        #[serde(default)]
        level: Option<LogLevel>,
        #[serde(default = "default_syslog_facility")]
        facility: String,
        #[serde(default = "default_syslog_ident")]
        ident: String,
    },
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: LogLevel::Info,
            format: LogFormat::Text,
            targets: vec![LogTarget::Console { level: None }],
            include_location: false,
            structured: true,
        }
    }
}

impl Default for LogLevel {
    fn default() -> Self {
        LogLevel::Info
    }
}

impl Default for LogFormat {
    fn default() -> Self {
        LogFormat::Text
    }
}

impl FromStr for LogLevel {
    type Err = String;
    
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "error" => Ok(LogLevel::Error),
            "warn" | "warning" => Ok(LogLevel::Warn),
            "info" => Ok(LogLevel::Info),
            "debug" => Ok(LogLevel::Debug),
            "trace" => Ok(LogLevel::Trace),
            _ => Err(format!("Invalid log level: {}", s)),
        }
    }
}

impl FromStr for LogFormat {
    type Err = String;
    
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "json" => Ok(LogFormat::Json),
            "text" => Ok(LogFormat::Text),
            "compact" => Ok(LogFormat::Compact),
            "pretty" => Ok(LogFormat::Pretty),
            _ => Err(format!("Invalid log format: {}", s)),
        }
    }
}

impl Validatable for LoggingConfig {
    fn validate(&self) -> ConfigResult<()> {
        // Validate targets
        for target in &self.targets {
            target.validate()?;
        }
        
        // Ensure at least one target
        if self.targets.is_empty() {
            return Err(self.validation_error("At least one log target must be configured"));
        }
        
        Ok(())
    }
    
    fn domain_name(&self) -> &'static str {
        "logging"
    }
}

impl Validatable for LogTarget {
    fn validate(&self) -> ConfigResult<()> {
        match self {
            LogTarget::Console { .. } => {
                // Console target is always valid
                Ok(())
            },
            LogTarget::File { path, max_size_bytes, max_files, .. } => {
                validate_required_string(path, "path", self.domain_name())?;
                
                if *max_size_bytes == 0 {
                    return Err(self.validation_error("max_size_bytes must be greater than 0"));
                }
                
                if *max_files == 0 {
                    return Err(self.validation_error("max_files must be greater than 0"));
                }
                
                Ok(())
            },
            LogTarget::Syslog { facility, ident, .. } => {
                validate_required_string(facility, "facility", self.domain_name())?;
                validate_required_string(ident, "ident", self.domain_name())?;
                
                // Validate syslog facility
                let valid_facilities = [
                    "kern", "user", "mail", "daemon", "auth", "syslog", "lpr", "news",
                    "uucp", "cron", "authpriv", "ftp", "local0", "local1", "local2",
                    "local3", "local4", "local5", "local6", "local7"
                ];
                validate_enum_choice(facility, &valid_facilities, "facility", self.domain_name())?;
                
                Ok(())
            }
        }
    }
    
    fn domain_name(&self) -> &'static str {
        "logging.target"
    }
}

// Default value functions
fn default_max_file_size() -> usize {
    10 * 1024 * 1024 // 10MB
}

fn default_max_files() -> usize {
    5
}

fn default_syslog_facility() -> String {
    "user".to_string()
}

fn default_syslog_ident() -> String {
    "ratchet".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_level_from_str() {
        assert_eq!(LogLevel::from_str("info").unwrap(), LogLevel::Info);
        assert_eq!(LogLevel::from_str("INFO").unwrap(), LogLevel::Info);
        assert_eq!(LogLevel::from_str("warn").unwrap(), LogLevel::Warn);
        assert_eq!(LogLevel::from_str("warning").unwrap(), LogLevel::Warn);
        assert!(LogLevel::from_str("invalid").is_err());
    }

    #[test]
    fn test_log_format_from_str() {
        assert_eq!(LogFormat::from_str("json").unwrap(), LogFormat::Json);
        assert_eq!(LogFormat::from_str("JSON").unwrap(), LogFormat::Json);
        assert!(LogFormat::from_str("invalid").is_err());
    }

    #[test]
    fn test_logging_config_defaults() {
        let config = LoggingConfig::default();
        assert_eq!(config.level, LogLevel::Info);
        assert_eq!(config.format, LogFormat::Text);
        assert!(!config.include_location);
        assert!(config.structured);
        assert_eq!(config.targets.len(), 1);
    }

    #[test]
    fn test_logging_config_validation() {
        let mut config = LoggingConfig::default();
        assert!(config.validate().is_ok());
        
        // Test empty targets
        config.targets.clear();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_log_target_validation() {
        // Console target
        let console = LogTarget::Console { level: None };
        assert!(console.validate().is_ok());
        
        // File target
        let file = LogTarget::File {
            path: "/var/log/ratchet.log".to_string(),
            level: None,
            max_size_bytes: 1024,
            max_files: 3,
        };
        assert!(file.validate().is_ok());
        
        // Invalid file target
        let invalid_file = LogTarget::File {
            path: String::new(),
            level: None,
            max_size_bytes: 0,
            max_files: 0,
        };
        assert!(invalid_file.validate().is_err());
        
        // Syslog target
        let syslog = LogTarget::Syslog {
            level: None,
            facility: "user".to_string(),
            ident: "ratchet".to_string(),
        };
        assert!(syslog.validate().is_ok());
    }
}