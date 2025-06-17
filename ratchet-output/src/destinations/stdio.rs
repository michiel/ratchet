//! Standard input/output destination implementation

use async_trait::async_trait;
use std::time::Instant;
use tokio::io::{AsyncWriteExt, BufWriter};

use crate::{
    destination::{DeliveryContext, DeliveryResult, OutputDestination, TaskOutput},
    errors::{DeliveryError, ValidationError},
    template::TemplateEngine,
    OutputFormat,
};

/// Standard streams for output
#[derive(Debug, Clone)]
pub enum StdStream {
    Stdout,
    Stderr,
}

impl Default for StdStream {
    fn default() -> Self {
        Self::Stdout
    }
}

/// Configuration for stdio destination
#[derive(Debug, Clone)]
pub struct StdioConfig {
    pub stream: StdStream,
    pub format: OutputFormat,
    pub include_metadata: bool,
    pub line_buffered: bool,
    pub prefix_template: Option<String>,
}

impl Default for StdioConfig {
    fn default() -> Self {
        Self {
            stream: StdStream::Stdout,
            format: OutputFormat::Json,
            include_metadata: false,
            line_buffered: true,
            prefix_template: None,
        }
    }
}

/// Standard output destination for writing output to stdout/stderr
#[derive(Debug)]
pub struct StdioDestination {
    config: StdioConfig,
    template_engine: TemplateEngine,
}

impl StdioDestination {
    pub fn new(config: StdioConfig, template_engine: TemplateEngine) -> Self {
        Self {
            config,
            template_engine,
        }
    }

    /// Format output data according to the configured format
    fn format_output(&self, output: &TaskOutput, context: &DeliveryContext) -> Result<Vec<u8>, DeliveryError> {
        let data = if self.config.include_metadata {
            // Include full task output with metadata
            serde_json::to_value(output).map_err(|e| DeliveryError::Serialization {
                format: "json".to_string(),
                error: e.to_string(),
            })?
        } else {
            // Just the output data
            output.output_data.clone()
        };

        let formatted_content = match &self.config.format {
            OutputFormat::Json => {
                serde_json::to_vec_pretty(&data).map_err(|e| DeliveryError::Serialization {
                    format: "json".to_string(),
                    error: e.to_string(),
                })?
            }
            OutputFormat::JsonCompact => {
                serde_json::to_vec(&data).map_err(|e| DeliveryError::Serialization {
                    format: "json_compact".to_string(),
                    error: e.to_string(),
                })?
            }
            #[cfg(feature = "yaml")]
            OutputFormat::Yaml => {
                serde_yaml::to_string(&data)
                    .map(|s| s.into_bytes())
                    .map_err(|e| DeliveryError::Serialization {
                        format: "yaml".to_string(),
                        error: e.to_string(),
                    })?
            }
            #[cfg(not(feature = "yaml"))]
            OutputFormat::Yaml => return Err(DeliveryError::Serialization {
                format: "yaml".to_string(),
                error: "YAML support not enabled".to_string(),
            }),
            OutputFormat::Raw => {
                if let serde_json::Value::String(s) = &data {
                    s.as_bytes().to_vec()
                } else {
                    data.to_string().into_bytes()
                }
            }
            OutputFormat::Template(template) => {
                let rendered = self
                    .template_engine
                    .render_json(template, &data)
                    .map_err(|e| match e {
                        DeliveryError::TemplateRender { template: t, error: err } => {
                            DeliveryError::TemplateRender { template: t, error: err }
                        }
                        _ => DeliveryError::TemplateRender {
                            template: template.clone(),
                            error: e.to_string(),
                        }
                    })?;
                rendered.into_bytes()
            }
            #[cfg(feature = "csv")]
            OutputFormat::Csv => self.convert_to_csv(&data)?,
            #[cfg(not(feature = "csv"))]
            OutputFormat::Csv => return Err(DeliveryError::Serialization {
                format: "csv".to_string(),
                error: "CSV support not enabled".to_string(),
            }),
        };

        // Add prefix if configured
        if let Some(prefix_template) = &self.config.prefix_template {
            let prefix = self
                .template_engine
                .render(prefix_template, &context.template_variables)?;
            let mut result = prefix.into_bytes();
            result.extend_from_slice(&formatted_content);
            Ok(result)
        } else {
            Ok(formatted_content)
        }
    }

    /// Convert JSON data to CSV format
    #[cfg(feature = "csv")]
    fn convert_to_csv(&self, data: &serde_json::Value) -> Result<Vec<u8>, DeliveryError> {
        match data {
            serde_json::Value::Array(arr) if !arr.is_empty() => {
                let mut wtr = csv::Writer::from_writer(Vec::new());

                // Extract headers from first object
                if let Some(serde_json::Value::Object(first_obj)) = arr.first() {
                    let headers: Vec<&String> = first_obj.keys().collect();
                    wtr.write_record(&headers)
                        .map_err(|e| DeliveryError::Serialization {
                            format: "csv".to_string(),
                            error: e.to_string(),
                        })?;

                    // Write data rows
                    for item in arr {
                        if let serde_json::Value::Object(obj) = item {
                            let values: Vec<String> = headers
                                .iter()
                                .map(|h| {
                                    obj.get(*h)
                                        .unwrap_or(&serde_json::Value::Null)
                                        .to_string()
                                })
                                .collect();
                            wtr.write_record(&values)
                                .map_err(|e| DeliveryError::Serialization {
                                    format: "csv".to_string(),
                                    error: e.to_string(),
                                })?;
                        }
                    }
                }

                wtr.into_inner().map_err(|e| DeliveryError::Serialization {
                    format: "csv".to_string(),
                    error: e.to_string(),
                })
            }
            serde_json::Value::Object(_) => {
                // Single object - treat as one row
                let mut wtr = csv::Writer::from_writer(Vec::new());
                
                if let serde_json::Value::Object(obj) = data {
                    let headers: Vec<&String> = obj.keys().collect();
                    wtr.write_record(&headers)
                        .map_err(|e| DeliveryError::Serialization {
                            format: "csv".to_string(),
                            error: e.to_string(),
                        })?;

                    let values: Vec<String> = headers
                        .iter()
                        .map(|h| {
                            obj.get(*h)
                                .unwrap_or(&serde_json::Value::Null)
                                .to_string()
                        })
                        .collect();
                    wtr.write_record(&values)
                        .map_err(|e| DeliveryError::Serialization {
                            format: "csv".to_string(),
                            error: e.to_string(),
                        })?;
                }

                wtr.into_inner().map_err(|e| DeliveryError::Serialization {
                    format: "csv".to_string(),
                    error: e.to_string(),
                })
            }
            _ => Err(DeliveryError::Serialization {
                format: "csv".to_string(),
                error: "CSV format requires an array of objects or a single object".to_string(),
            }),
        }
    }

    /// Write data to the configured stream
    async fn write_to_stream(&self, data: &[u8]) -> Result<(), DeliveryError> {
        match self.config.stream {
            StdStream::Stdout => {
                let mut stdout = tokio::io::stdout();
                if self.config.line_buffered {
                    let mut writer = BufWriter::new(&mut stdout);
                    writer.write_all(data).await
                        .map_err(|e| DeliveryError::Stdio {
                            stream: "stdout".to_string(),
                            error: e.to_string(),
                        })?;
                    writer.write_all(b"\n").await
                        .map_err(|e| DeliveryError::Stdio {
                            stream: "stdout".to_string(),
                            error: e.to_string(),
                        })?;
                    writer.flush().await
                        .map_err(|e| DeliveryError::Stdio {
                            stream: "stdout".to_string(),
                            error: e.to_string(),
                        })?;
                } else {
                    stdout.write_all(data).await
                        .map_err(|e| DeliveryError::Stdio {
                            stream: "stdout".to_string(),
                            error: e.to_string(),
                        })?;
                    stdout.write_all(b"\n").await
                        .map_err(|e| DeliveryError::Stdio {
                            stream: "stdout".to_string(),
                            error: e.to_string(),
                        })?;
                    stdout.flush().await
                        .map_err(|e| DeliveryError::Stdio {
                            stream: "stdout".to_string(),
                            error: e.to_string(),
                        })?;
                }
            }
            StdStream::Stderr => {
                let mut stderr = tokio::io::stderr();
                if self.config.line_buffered {
                    let mut writer = BufWriter::new(&mut stderr);
                    writer.write_all(data).await
                        .map_err(|e| DeliveryError::Stdio {
                            stream: "stderr".to_string(),
                            error: e.to_string(),
                        })?;
                    writer.write_all(b"\n").await
                        .map_err(|e| DeliveryError::Stdio {
                            stream: "stderr".to_string(),
                            error: e.to_string(),
                        })?;
                    writer.flush().await
                        .map_err(|e| DeliveryError::Stdio {
                            stream: "stderr".to_string(),
                            error: e.to_string(),
                        })?;
                } else {
                    stderr.write_all(data).await
                        .map_err(|e| DeliveryError::Stdio {
                            stream: "stderr".to_string(),
                            error: e.to_string(),
                        })?;
                    stderr.write_all(b"\n").await
                        .map_err(|e| DeliveryError::Stdio {
                            stream: "stderr".to_string(),
                            error: e.to_string(),
                        })?;
                    stderr.flush().await
                        .map_err(|e| DeliveryError::Stdio {
                            stream: "stderr".to_string(),
                            error: e.to_string(),
                        })?;
                }
            }
        }
        Ok(())
    }
}

#[async_trait]
impl OutputDestination for StdioDestination {
    async fn deliver(
        &self,
        output: &TaskOutput,
        context: &DeliveryContext,
    ) -> Result<DeliveryResult, DeliveryError> {
        let start_time = Instant::now();

        // Format the output data
        let formatted_data = self.format_output(output, context)?;
        let size_bytes = formatted_data.len() as u64;

        // Write to the configured stream
        self.write_to_stream(&formatted_data).await?;

        let delivery_time = start_time.elapsed();
        let stream_name = match self.config.stream {
            StdStream::Stdout => "stdout",
            StdStream::Stderr => "stderr",
        };

        Ok(DeliveryResult::success(
            format!("stdio-{}", stream_name),
            delivery_time,
            size_bytes,
            Some(format!("Written to {}", stream_name)),
        ))
    }

    fn validate_config(&self) -> Result<(), ValidationError> {
        // Validate prefix template if provided
        if let Some(prefix_template) = &self.config.prefix_template {
            self.template_engine
                .validate(prefix_template)
                .map_err(|e| ValidationError::InvalidTemplate(e.to_string()))?;
        }

        Ok(())
    }

    fn destination_type(&self) -> &'static str {
        "stdio"
    }

    fn supports_retry(&self) -> bool {
        false // Stdio writes are typically immediate and don't benefit from retries
    }

    fn estimated_delivery_time(&self) -> std::time::Duration {
        std::time::Duration::from_millis(10) // Stdio writes are very fast
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::template::TemplateEngine;
    use crate::destination::TaskOutput;
    use serde_json::json;
    use chrono::Utc;
    use std::collections::HashMap;
    use std::time::Duration;

    #[test]
    fn test_stdio_config_defaults() {
        let config = StdioConfig::default();
        assert!(matches!(config.stream, StdStream::Stdout));
        assert!(matches!(config.format, OutputFormat::Json));
        assert!(!config.include_metadata);
        assert!(config.line_buffered);
        assert!(config.prefix_template.is_none());
    }

    #[test]
    fn test_stdio_destination_validation() {
        let template_engine = TemplateEngine::new();
        
        // Test valid config
        let valid_config = StdioConfig {
            stream: StdStream::Stdout,
            format: OutputFormat::Json,
            include_metadata: false,
            line_buffered: true,
            prefix_template: Some("[{{timestamp}}] ".to_string()),
        };
        
        let destination = StdioDestination::new(valid_config, template_engine.clone());
        assert!(destination.validate_config().is_ok());
        
        // Test invalid template
        let invalid_config = StdioConfig {
            stream: StdStream::Stderr,
            format: OutputFormat::Json,
            include_metadata: false,
            line_buffered: true,
            prefix_template: Some("{{invalid_template".to_string()), // Missing closing brace
        };
        
        let destination = StdioDestination::new(invalid_config, template_engine);
        assert!(destination.validate_config().is_err());
    }

    #[tokio::test]
    async fn test_stdio_output_formatting() {
        let template_engine = TemplateEngine::new();
        let config = StdioConfig {
            stream: StdStream::Stdout,
            format: OutputFormat::Json,
            include_metadata: false,
            line_buffered: false,
            prefix_template: None,
        };
        
        let destination = StdioDestination::new(config, template_engine);
        
        // Create test data
        let output = TaskOutput {
            job_id: 1,
            task_id: 1,
            execution_id: 1,
            output_data: json!({
                "status": "success",
                "result": 42,
                "message": "Test output"
            }),
            metadata: HashMap::new(),
            completed_at: Utc::now(),
            execution_duration: Duration::from_secs(1),
        };
        
        let context = DeliveryContext::default();
        
        // Test format_output
        let formatted = destination.format_output(&output, &context).unwrap();
        assert!(!formatted.is_empty());
        
        // Verify it's valid JSON
        let parsed: serde_json::Value = serde_json::from_slice(&formatted).unwrap();
        assert_eq!(parsed["status"], "success");
        assert_eq!(parsed["result"], 42);
    }

    #[tokio::test]
    async fn test_stdio_with_metadata() {
        let template_engine = TemplateEngine::new();
        let config = StdioConfig {
            stream: StdStream::Stdout,
            format: OutputFormat::Json,
            include_metadata: true, // Include full metadata
            line_buffered: false,
            prefix_template: None,
        };
        
        let destination = StdioDestination::new(config, template_engine);
        
        // Create test data
        let output = TaskOutput {
            job_id: 123,
            task_id: 456,
            execution_id: 789,
            output_data: json!({"result": "test"}),
            metadata: HashMap::new(),
            completed_at: Utc::now(),
            execution_duration: Duration::from_secs(5),
        };
        
        let context = DeliveryContext::default();
        
        // Test format_output with metadata
        let formatted = destination.format_output(&output, &context).unwrap();
        let parsed: serde_json::Value = serde_json::from_slice(&formatted).unwrap();
        
        // Verify metadata is included
        assert_eq!(parsed["job_id"], 123);
        assert_eq!(parsed["task_id"], 456);
        assert_eq!(parsed["execution_id"], 789);
        assert_eq!(parsed["output_data"]["result"], "test");
    }

    #[tokio::test]
    async fn test_stdio_with_prefix() {
        let template_engine = TemplateEngine::new();
        let config = StdioConfig {
            stream: StdStream::Stdout,
            format: OutputFormat::JsonCompact,
            include_metadata: false,
            line_buffered: false,
            prefix_template: Some("[Job {{job_id}}] ".to_string()),
        };
        
        let destination = StdioDestination::new(config, template_engine);
        
        let output = TaskOutput {
            job_id: 999,
            task_id: 1,
            execution_id: 1,
            output_data: json!({"result": "test"}),
            metadata: HashMap::new(),
            completed_at: Utc::now(),
            execution_duration: Duration::from_secs(1),
        };
        
        let mut context = DeliveryContext::default();
        context.template_variables.insert("job_id".to_string(), "999".to_string());
        
        let formatted = destination.format_output(&output, &context).unwrap();
        let formatted_str = String::from_utf8(formatted).unwrap();
        
        assert!(formatted_str.starts_with("[Job 999] "));
        assert!(formatted_str.contains("{\"result\":\"test\"}"));
    }
}