use super::{LoggingConfig, init_logger};
use crate::errors::RatchetError;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

/// Initialize logging from configuration
pub fn init_logging_from_config(config: &LoggingConfig) -> Result<(), RatchetError> {
    // If no sinks are configured, fall back to simple tracing
    if config.sinks.is_empty() {
        return init_simple_tracing(&config.level.to_string());
    }

    // Check if only console sink is configured - use tracing for simplicity
    if config.sinks.len() == 1 {
        if let Some(sink) = config.sinks.first() {
            if let super::config::SinkConfig::Console { level, use_json } = sink {
                if !use_json {
                    return init_simple_tracing(&level.to_string());
                }
            }
        }
    }

    // Use structured logging for complex configurations
    let logger = config.build_logger()
        .map_err(|e| RatchetError::Configuration(format!("Failed to build logger: {}", e)))?;
    
    init_logger(logger)
        .map_err(|e| RatchetError::Configuration(format!("Failed to initialize logger: {}", e)))?;

    Ok(())
}

/// Initialize simple tracing for basic console output
pub fn init_simple_tracing(log_level: &str) -> Result<(), RatchetError> {
    let env_filter = EnvFilter::try_new(log_level)
        .or_else(|_| EnvFilter::try_from_default_env())
        .unwrap_or_else(|_| EnvFilter::new("info"));
    
    tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .init();
    
    Ok(())
}

/// Initialize logging with both structured and tracing layers
pub fn init_hybrid_logging(config: &LoggingConfig) -> Result<(), RatchetError> {
    // Create tracing subscriber for console output
    let console_layer = tracing_subscriber::fmt::layer()
        .with_target(false)
        .with_thread_ids(true)
        .with_line_number(true);

    // Create env filter
    let env_filter = EnvFilter::try_new(config.level.to_string())
        .or_else(|_| EnvFilter::try_from_default_env())
        .unwrap_or_else(|_| EnvFilter::new("info"));

    // Initialize subscriber
    tracing_subscriber::registry()
        .with(env_filter)
        .with(console_layer)
        .init();

    // Also initialize structured logger for file outputs
    if config.sinks.iter().any(|s| !matches!(s, super::config::SinkConfig::Console { .. })) {
        let logger = config.build_logger()
            .map_err(|e| RatchetError::Configuration(format!("Failed to build structured logger: {}", e)))?;
        
        init_logger(logger)
            .map_err(|e| RatchetError::Configuration(format!("Failed to initialize structured logger: {}", e)))?;
    }

    Ok(())
}