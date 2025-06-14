//! Shared error sanitization utilities

use ratchet_core::validation::error_sanitization::{ErrorSanitizer, ErrorSanitizationConfig};
use std::sync::{Arc, RwLock};
use std::collections::HashMap;

/// Shared error sanitizer with runtime configuration support
pub struct SharedErrorSanitizer {
    sanitizer: Arc<RwLock<ErrorSanitizer>>,
}

impl Default for SharedErrorSanitizer {
    fn default() -> Self {
        Self {
            sanitizer: Arc::new(RwLock::new(ErrorSanitizer::default())),
        }
    }
}

impl SharedErrorSanitizer {
    /// Create a new shared sanitizer with configuration
    pub fn new(config: ErrorSanitizationConfig) -> Self {
        Self {
            sanitizer: Arc::new(RwLock::new(ErrorSanitizer::new(config))),
        }
    }
    
    /// Update the sanitizer configuration at runtime
    pub fn update_config(&self, config: ErrorSanitizationConfig) {
        if let Ok(mut sanitizer) = self.sanitizer.write() {
            *sanitizer = ErrorSanitizer::new(config);
        }
    }
    
    /// Add custom error mappings
    pub fn add_custom_mappings(&self, mappings: HashMap<String, String>) {
        if let Ok(sanitizer) = self.sanitizer.read() {
            // Create new config with updated mappings
            let mut config = ErrorSanitizationConfig::default();
            config.custom_mappings.extend(mappings);
            drop(sanitizer);
            
            // Update with new config
            if let Ok(mut sanitizer) = self.sanitizer.write() {
                *sanitizer = ErrorSanitizer::new(config);
            }
        }
    }
    
    /// Sanitize an error message
    pub fn sanitize_error<E: std::error::Error>(&self, error: &E) -> ratchet_core::validation::error_sanitization::SanitizedError {
        if let Ok(sanitizer) = self.sanitizer.read() {
            sanitizer.sanitize_error(error)
        } else {
            // Fallback if lock fails
            let fallback_sanitizer = ErrorSanitizer::default();
            fallback_sanitizer.sanitize_error(error)
        }
    }
    
    /// Sanitize a message string
    pub fn sanitize_message(&self, message: &str) -> ratchet_core::validation::error_sanitization::SanitizedError {
        if let Ok(sanitizer) = self.sanitizer.read() {
            sanitizer.sanitize_message(message)
        } else {
            // Fallback if lock fails
            let fallback_sanitizer = ErrorSanitizer::default();
            fallback_sanitizer.sanitize_message(message)
        }
    }
}

/// Builder for creating sanitization configurations
pub struct SanitizationConfigBuilder {
    config: ErrorSanitizationConfig,
}

impl Default for SanitizationConfigBuilder {
    fn default() -> Self {
        Self {
            config: ErrorSanitizationConfig::default(),
        }
    }
}

impl SanitizationConfigBuilder {
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Include error codes in sanitized messages
    pub fn include_error_codes(mut self, include: bool) -> Self {
        self.config.include_error_codes = include;
        self
    }
    
    /// Include safe context information
    pub fn include_safe_context(mut self, include: bool) -> Self {
        self.config.include_safe_context = include;
        self
    }
    
    /// Set maximum message length
    pub fn max_message_length(mut self, length: usize) -> Self {
        self.config.max_message_length = length;
        self
    }
    
    /// Add custom error mappings
    pub fn add_custom_mapping(mut self, pattern: impl Into<String>, replacement: impl Into<String>) -> Self {
        self.config.custom_mappings.insert(pattern.into(), replacement.into());
        self
    }
    
    /// Add multiple custom mappings
    pub fn add_custom_mappings(mut self, mappings: HashMap<String, String>) -> Self {
        self.config.custom_mappings.extend(mappings);
        self
    }
    
    /// Build the configuration
    pub fn build(self) -> ErrorSanitizationConfig {
        self.config
    }
}

/// Pre-configured sanitization setups for different environments
pub struct SanitizationPresets;

impl SanitizationPresets {
    /// Development environment - less restrictive, more context
    pub fn development() -> ErrorSanitizationConfig {
        SanitizationConfigBuilder::new()
            .include_error_codes(true)
            .include_safe_context(true)
            .max_message_length(500)
            .build()
    }
    
    /// Production environment - strict sanitization
    pub fn production() -> ErrorSanitizationConfig {
        SanitizationConfigBuilder::new()
            .include_error_codes(true)
            .include_safe_context(false)
            .max_message_length(200)
            .add_custom_mapping("database", "A database issue occurred")
            .add_custom_mapping("connection", "A connection issue occurred")
            .add_custom_mapping("timeout", "The operation timed out")
            .build()
    }
    
    /// Testing environment - minimal sanitization for debugging
    pub fn testing() -> ErrorSanitizationConfig {
        SanitizationConfigBuilder::new()
            .include_error_codes(true)
            .include_safe_context(true)
            .max_message_length(1000)
            .build()
    }
    
    /// Security-focused - maximum sanitization
    pub fn security_focused() -> ErrorSanitizationConfig {
        SanitizationConfigBuilder::new()
            .include_error_codes(false)
            .include_safe_context(false)
            .max_message_length(100)
            .add_custom_mapping("failed", "An error occurred")
            .add_custom_mapping("error", "An error occurred")
            .add_custom_mapping("exception", "An error occurred")
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;
    
    #[test]
    fn test_shared_sanitizer() {
        let sanitizer = SharedErrorSanitizer::default();
        let error = io::Error::new(io::ErrorKind::NotFound, "file not found: /secret/config.yaml");
        
        let sanitized = sanitizer.sanitize_error(&error);
        assert!(!sanitized.message.contains("/secret/"));
    }
    
    #[test]
    fn test_config_builder() {
        let config = SanitizationConfigBuilder::new()
            .include_error_codes(false)
            .max_message_length(50)
            .add_custom_mapping("test", "custom message")
            .build();
        
        assert!(!config.include_error_codes);
        assert_eq!(config.max_message_length, 50);
        assert_eq!(config.custom_mappings.get("test"), Some(&"custom message".to_string()));
    }
    
    #[test]
    fn test_sanitization_presets() {
        let dev_config = SanitizationPresets::development();
        let prod_config = SanitizationPresets::production();
        let security_config = SanitizationPresets::security_focused();
        
        assert!(dev_config.include_safe_context);
        assert!(!prod_config.include_safe_context);
        assert!(!security_config.include_error_codes);
        
        assert!(dev_config.max_message_length > prod_config.max_message_length);
        assert!(prod_config.max_message_length > security_config.max_message_length);
    }
    
    #[test]
    fn test_runtime_config_update() {
        let sanitizer = SharedErrorSanitizer::default();
        
        let initial_message = sanitizer.sanitize_message("database error occurred");
        
        // Update config with custom mapping
        let new_config = SanitizationConfigBuilder::new()
            .add_custom_mapping("database error", "system issue")
            .build();
        
        sanitizer.update_config(new_config);
        
        let updated_message = sanitizer.sanitize_message("database error occurred");
        assert_ne!(initial_message.message, updated_message.message);
        assert_eq!(updated_message.message, "system issue");
    }
}