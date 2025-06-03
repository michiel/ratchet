//! Task execution configuration

use serde::{Deserialize, Serialize};
use std::time::Duration;
use crate::validation::{Validatable, validate_required_string, validate_positive};
use crate::error::ConfigResult;

/// Task execution configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ExecutionConfig {
    /// JavaScript variable names used for fetch operations
    #[serde(default)]
    pub fetch_variables: FetchVariables,
    
    /// Maximum execution time for JavaScript tasks
    #[serde(with = "crate::domains::utils::serde_duration", default = "default_max_execution_duration")]
    pub max_execution_duration: Duration,
    
    /// Whether to validate schemas during execution
    #[serde(default = "crate::domains::utils::default_true")]
    pub validate_schemas: bool,
    
    /// Maximum number of concurrent task executions
    #[serde(default = "default_max_concurrent_tasks")]
    pub max_concurrent_tasks: usize,
    
    /// Task execution timeout grace period (for cleanup)
    #[serde(with = "crate::domains::utils::serde_duration", default = "default_timeout_grace_period")]
    pub timeout_grace_period: Duration,
}

/// JavaScript fetch variables configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct FetchVariables {
    /// Variable name for fetch URL
    #[serde(default = "default_url_var")]
    pub url_var: String,
    
    /// Variable name for fetch parameters
    #[serde(default = "default_params_var")]
    pub params_var: String,
    
    /// Variable name for fetch body
    #[serde(default = "default_body_var")]
    pub body_var: String,
    
    /// Variable name for HTTP result
    #[serde(default = "default_result_var")]
    pub result_var: String,
    
    /// Variable name for temporary result
    #[serde(default = "default_temp_result_var")]
    pub temp_result_var: String,
}

impl Default for ExecutionConfig {
    fn default() -> Self {
        Self {
            fetch_variables: FetchVariables::default(),
            max_execution_duration: default_max_execution_duration(),
            validate_schemas: true,
            max_concurrent_tasks: default_max_concurrent_tasks(),
            timeout_grace_period: default_timeout_grace_period(),
        }
    }
}

impl Default for FetchVariables {
    fn default() -> Self {
        Self {
            url_var: default_url_var(),
            params_var: default_params_var(),
            body_var: default_body_var(),
            result_var: default_result_var(),
            temp_result_var: default_temp_result_var(),
        }
    }
}

impl Validatable for ExecutionConfig {
    fn validate(&self) -> ConfigResult<()> {
        // Validate duration
        validate_positive(
            self.max_execution_duration.as_secs(),
            "max_execution_duration",
            self.domain_name()
        )?;
        
        validate_positive(
            self.timeout_grace_period.as_secs(),
            "timeout_grace_period",
            self.domain_name()
        )?;
        
        validate_positive(
            self.max_concurrent_tasks,
            "max_concurrent_tasks",
            self.domain_name()
        )?;
        
        // Validate fetch variables
        self.fetch_variables.validate()?;
        
        Ok(())
    }
    
    fn domain_name(&self) -> &'static str {
        "execution"
    }
}

impl Validatable for FetchVariables {
    fn validate(&self) -> ConfigResult<()> {
        validate_required_string(&self.url_var, "url_var", self.domain_name())?;
        validate_required_string(&self.params_var, "params_var", self.domain_name())?;
        validate_required_string(&self.body_var, "body_var", self.domain_name())?;
        validate_required_string(&self.result_var, "result_var", self.domain_name())?;
        validate_required_string(&self.temp_result_var, "temp_result_var", self.domain_name())?;
        
        Ok(())
    }
    
    fn domain_name(&self) -> &'static str {
        "execution.fetch_variables"
    }
}

// Default value functions
fn default_max_execution_duration() -> Duration {
    Duration::from_secs(300) // 5 minutes
}

fn default_max_concurrent_tasks() -> usize {
    10
}

fn default_timeout_grace_period() -> Duration {
    Duration::from_secs(5)
}

fn default_url_var() -> String {
    "__fetch_url".to_string()
}

fn default_params_var() -> String {
    "__fetch_params".to_string()
}

fn default_body_var() -> String {
    "__fetch_body".to_string()
}

fn default_result_var() -> String {
    "__http_result".to_string()
}

fn default_temp_result_var() -> String {
    "__temp_result".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execution_config_defaults() {
        let config = ExecutionConfig::default();
        assert_eq!(config.max_execution_duration, Duration::from_secs(300));
        assert!(config.validate_schemas);
        assert_eq!(config.max_concurrent_tasks, 10);
    }

    #[test]
    fn test_execution_config_validation() {
        let mut config = ExecutionConfig::default();
        assert!(config.validate().is_ok());
        
        // Test invalid duration
        config.max_execution_duration = Duration::from_secs(0);
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_fetch_variables_validation() {
        let mut vars = FetchVariables::default();
        assert!(vars.validate().is_ok());
        
        // Test empty variable name
        vars.url_var = String::new();
        assert!(vars.validate().is_err());
    }
}