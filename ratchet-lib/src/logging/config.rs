use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;
use super::{LogLevel, LoggerBuilder, logger::LogSink};
use super::sinks::{ConsoleSink, FileSink, BufferedSink};
use super::enrichment::{SystemEnricher, ProcessEnricher, TaskContextEnricher, ExecutionContextEnricher};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LoggingConfig {
    /// Minimum log level
    #[serde(default = "default_log_level")]
    pub level: LogLevel,
    
    /// Output format
    #[serde(default = "default_format")]
    pub format: LogFormat,
    
    /// Log sinks configuration
    #[serde(default)]
    pub sinks: Vec<SinkConfig>,
    
    /// Enrichment configuration
    #[serde(default)]
    pub enrichment: EnrichmentConfig,
    
    /// Sampling configuration
    #[serde(default)]
    pub sampling: SamplingConfig,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogFormat {
    Json,
    Pretty,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum SinkConfig {
    Console {
        #[serde(default = "default_log_level")]
        level: LogLevel,
        #[serde(default)]
        use_json: bool,
    },
    File {
        path: PathBuf,
        #[serde(default = "default_log_level")]
        level: LogLevel,
        #[serde(default)]
        rotation: Option<RotationConfig>,
        #[serde(default)]
        buffered: Option<BufferConfig>,
    },
    Database {
        table: String,
        #[serde(default)]
        buffer_size: usize,
        #[serde(default)]
        flush_interval: Duration,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RotationConfig {
    /// Maximum file size before rotation (e.g., "100MB")
    pub max_size: String,
    /// Maximum age before rotation (e.g., "7d")
    #[serde(default)]
    pub max_age: Option<String>,
    /// Maximum number of rotated files to keep
    #[serde(default)]
    pub max_files: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BufferConfig {
    #[serde(default = "default_buffer_size")]
    pub size: usize,
    #[serde(with = "humantime_serde", default = "default_flush_interval")]
    pub flush_interval: Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct EnrichmentConfig {
    pub system_info: bool,
    pub process_info: bool,
    pub task_context: bool,
    pub execution_context: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SamplingConfig {
    /// Sample rate for error level logs (0.0 - 1.0)
    pub error_rate: f64,
    /// Sample rate for warn level logs
    pub warn_rate: f64,
    /// Sample rate for info level logs
    pub info_rate: f64,
    /// Sample rate for debug level logs
    pub debug_rate: f64,
    /// Sample rate for trace level logs
    pub trace_rate: f64,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: default_log_level(),
            format: default_format(),
            sinks: vec![
                SinkConfig::Console {
                    level: LogLevel::Info,
                    use_json: false,
                }
            ],
            enrichment: EnrichmentConfig::default(),
            sampling: SamplingConfig::default(),
        }
    }
}

impl Default for EnrichmentConfig {
    fn default() -> Self {
        Self {
            system_info: true,
            process_info: true,
            task_context: false,
            execution_context: false,
        }
    }
}

impl Default for SamplingConfig {
    fn default() -> Self {
        Self {
            error_rate: 1.0,
            warn_rate: 1.0,
            info_rate: 1.0,
            debug_rate: 0.1,
            trace_rate: 0.01,
        }
    }
}

impl LoggingConfig {
    /// Build a logger from this configuration
    pub fn build_logger(&self) -> Result<Arc<dyn super::StructuredLogger>, ConfigError> {
        let mut builder = LoggerBuilder::new()
            .with_min_level(self.level);

        // Add sinks
        for sink_config in &self.sinks {
            let sink = self.create_sink(sink_config)?;
            builder = builder.add_sink(sink);
        }

        // Add enrichers
        if self.enrichment.system_info {
            builder = builder.add_enricher(Box::new(SystemEnricher::new()));
        }
        if self.enrichment.process_info {
            builder = builder.add_enricher(Box::new(ProcessEnricher::new()));
        }
        if self.enrichment.task_context {
            builder = builder.add_enricher(Box::new(TaskContextEnricher::new()));
        }
        if self.enrichment.execution_context {
            builder = builder.add_enricher(Box::new(ExecutionContextEnricher::new()));
        }

        // TODO: Add sampling wrapper when implemented

        Ok(builder.build())
    }

    fn create_sink(&self, config: &SinkConfig) -> Result<Arc<dyn LogSink>, ConfigError> {
        match config {
            SinkConfig::Console { level, use_json } => {
                let mut sink = ConsoleSink::new(*level);
                if *use_json {
                    sink = sink.json_format();
                }
                Ok(Arc::new(sink))
            }
            SinkConfig::File { path, level, rotation, buffered } => {
                let mut file_sink = FileSink::new(path, *level)
                    .map_err(|e| ConfigError::SinkCreation(format!("Failed to create file sink: {}", e)))?;

                if let Some(rotation_config) = rotation {
                    let max_size = parse_size(&rotation_config.max_size)?;
                    file_sink = file_sink.with_rotation(max_size);
                }

                let sink: Arc<dyn LogSink> = Arc::new(file_sink);

                if let Some(buffer_config) = buffered {
                    Ok(Arc::new(BufferedSink::new(
                        sink,
                        buffer_config.size,
                        buffer_config.flush_interval,
                    )))
                } else {
                    Ok(sink)
                }
            }
            SinkConfig::Database { .. } => {
                Err(ConfigError::NotImplemented("Database sink not yet implemented".to_string()))
            }
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Failed to create sink: {0}")]
    SinkCreation(String),
    
    #[error("Invalid size format: {0}")]
    InvalidSize(String),
    
    #[error("Not implemented: {0}")]
    NotImplemented(String),
}

// Helper functions
fn default_log_level() -> LogLevel {
    LogLevel::Info
}

fn default_format() -> LogFormat {
    LogFormat::Pretty
}

fn default_buffer_size() -> usize {
    1000
}

fn default_flush_interval() -> Duration {
    Duration::from_secs(5)
}

fn parse_size(size_str: &str) -> Result<u64, ConfigError> {
    let size_str = size_str.trim().to_uppercase();
    
    if let Some(kb_str) = size_str.strip_suffix("KB") {
        kb_str.trim().parse::<u64>()
            .map(|n| n * 1024)
            .map_err(|_| ConfigError::InvalidSize(size_str))
    } else if let Some(mb_str) = size_str.strip_suffix("MB") {
        mb_str.trim().parse::<u64>()
            .map(|n| n * 1024 * 1024)
            .map_err(|_| ConfigError::InvalidSize(size_str))
    } else if let Some(gb_str) = size_str.strip_suffix("GB") {
        gb_str.trim().parse::<u64>()
            .map(|n| n * 1024 * 1024 * 1024)
            .map_err(|_| ConfigError::InvalidSize(size_str))
    } else {
        size_str.parse::<u64>()
            .map_err(|_| ConfigError::InvalidSize(size_str))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_size() {
        assert_eq!(parse_size("100").unwrap(), 100);
        assert_eq!(parse_size("10KB").unwrap(), 10 * 1024);
        assert_eq!(parse_size("10 KB").unwrap(), 10 * 1024);
        assert_eq!(parse_size("100MB").unwrap(), 100 * 1024 * 1024);
        assert_eq!(parse_size("1GB").unwrap(), 1024 * 1024 * 1024);
    }

    #[test]
    fn test_default_config() {
        let config = LoggingConfig::default();
        assert_eq!(config.level, LogLevel::Info);
        assert!(matches!(config.format, LogFormat::Pretty));
        assert_eq!(config.sinks.len(), 1);
    }

    #[test]
    fn test_config_deserialization() {
        let yaml = r#"
level: debug
format: json
sinks:
  - type: console
    level: warn
    use_json: true
  - type: file
    path: /var/log/app.log
    level: info
    rotation:
      max_size: 100MB
      max_age: 7d
enrichment:
  system_info: true
  process_info: false
sampling:
  error_rate: 1.0
  info_rate: 0.5
"#;
        
        let config: LoggingConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.level, LogLevel::Debug);
        assert_eq!(config.sinks.len(), 2);
        assert_eq!(config.sampling.info_rate, 0.5);
    }
}