//! Filesystem output destination implementation

use async_trait::async_trait;
use std::path::Path;
use std::time::Instant;
use tokio::fs;

use crate::output::{
    destination::{DeliveryContext, DeliveryResult, OutputDestination, TaskOutput},
    errors::{DeliveryError, ValidationError},
    template::TemplateEngine,
    OutputFormat,
};

/// Configuration for filesystem destination
#[derive(Debug, Clone)]
pub struct FilesystemConfig {
    pub path_template: String,
    pub format: OutputFormat,
    pub permissions: u32,
    pub create_dirs: bool,
    pub overwrite: bool,
    pub backup_existing: bool,
}

/// Filesystem destination for writing output to files
#[derive(Debug)]
pub struct FilesystemDestination {
    config: FilesystemConfig,
    template_engine: TemplateEngine,
}

impl FilesystemDestination {
    pub fn new(config: FilesystemConfig, template_engine: TemplateEngine) -> Self {
        Self {
            config,
            template_engine,
        }
    }

    /// Format output data according to the configured format
    fn format_output(&self, data: &serde_json::Value) -> Result<Vec<u8>, DeliveryError> {
        match &self.config.format {
            OutputFormat::Json => {
                serde_json::to_vec_pretty(data).map_err(|e| DeliveryError::Serialization {
                    format: "json".to_string(),
                    error: e.to_string(),
                })
            }
            OutputFormat::JsonCompact => {
                serde_json::to_vec(data).map_err(|e| DeliveryError::Serialization {
                    format: "json_compact".to_string(),
                    error: e.to_string(),
                })
            }
            OutputFormat::Yaml => {
                serde_yaml::to_string(data)
                    .map(|s| s.into_bytes())
                    .map_err(|e| DeliveryError::Serialization {
                        format: "yaml".to_string(),
                        error: e.to_string(),
                    })
            }
            OutputFormat::Raw => {
                if let serde_json::Value::String(s) = data {
                    Ok(s.as_bytes().to_vec())
                } else {
                    Ok(data.to_string().into_bytes())
                }
            }
            OutputFormat::Template(template) => {
                let rendered = self
                    .template_engine
                    .render(template, &std::collections::HashMap::new())
                    .map_err(|e| DeliveryError::TemplateRender {
                        template: template.clone(),
                        error: e.to_string(),
                    })?;
                Ok(rendered.into_bytes())
            }
            OutputFormat::Csv => self.convert_to_csv(data),
        }
    }

    /// Convert JSON data to CSV format
    #[cfg(feature = "output")]
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
                                        .map(|v| match v {
                                            serde_json::Value::String(s) => s.clone(),
                                            serde_json::Value::Null => String::new(),
                                            _ => v.to_string().trim_matches('"').to_string(),
                                        })
                                        .unwrap_or_default()
                                })
                                .collect();
                            wtr.write_record(&values).map_err(|e| {
                                DeliveryError::Serialization {
                                    format: "csv".to_string(),
                                    error: e.to_string(),
                                }
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
                // Single object - convert to single row CSV
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
                                .map(|v| match v {
                                    serde_json::Value::String(s) => s.clone(),
                                    serde_json::Value::Null => String::new(),
                                    _ => v.to_string().trim_matches('"').to_string(),
                                })
                                .unwrap_or_default()
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
            _ => {
                // Fallback to JSON for other data types
                serde_json::to_vec_pretty(data).map_err(|e| DeliveryError::Serialization {
                    format: "csv_fallback".to_string(),
                    error: e.to_string(),
                })
            }
        }
    }

    /// Convert JSON data to CSV format (fallback when output feature disabled)
    #[cfg(not(feature = "output"))]
    fn convert_to_csv(&self, data: &serde_json::Value) -> Result<Vec<u8>, DeliveryError> {
        // Fallback to JSON when CSV feature is not available
        serde_json::to_vec_pretty(data).map_err(|e| DeliveryError::Serialization {
            format: "csv_fallback_json".to_string(),
            error: format!("CSV feature not enabled, falling back to JSON: {}", e),
        })
    }

    /// Backup existing file with timestamp
    async fn backup_existing_file(&self, file_path: &str) -> Result<(), DeliveryError> {
        let backup_path = format!(
            "{}.backup.{}",
            file_path,
            chrono::Utc::now().format("%Y%m%d_%H%M%S")
        );

        fs::copy(file_path, &backup_path)
            .await
            .map_err(|e| DeliveryError::Filesystem {
                path: backup_path,
                operation: "backup".to_string(),
                error: e.to_string(),
            })?;

        Ok(())
    }
}

#[async_trait]
impl OutputDestination for FilesystemDestination {
    async fn deliver(
        &self,
        output: &TaskOutput,
        context: &DeliveryContext,
    ) -> Result<DeliveryResult, DeliveryError> {
        let start_time = Instant::now();

        // 1. Render path template
        let file_path = self
            .template_engine
            .render(&self.config.path_template, &context.template_variables)
            .map_err(|e| DeliveryError::TemplateRender {
                template: self.config.path_template.clone(),
                error: e.to_string(),
            })?;

        // 2. Format output data
        let formatted_output = self.format_output(&output.output_data)?;

        // 3. Create parent directories if needed
        if self.config.create_dirs {
            if let Some(parent) = Path::new(&file_path).parent() {
                fs::create_dir_all(parent)
                    .await
                    .map_err(|e| DeliveryError::Filesystem {
                        path: file_path.clone(),
                        operation: "create_dirs".to_string(),
                        error: e.to_string(),
                    })?;
            }
        }

        // 4. Handle existing file
        if Path::new(&file_path).exists() && !self.config.overwrite {
            if self.config.backup_existing {
                self.backup_existing_file(&file_path).await?;
            } else {
                return Err(DeliveryError::FileExists { path: file_path });
            }
        }

        // 5. Write file atomically
        let temp_path = format!("{}.tmp.{}", file_path, std::process::id());
        fs::write(&temp_path, &formatted_output)
            .await
            .map_err(|e| DeliveryError::Filesystem {
                path: temp_path.clone(),
                operation: "write".to_string(),
                error: e.to_string(),
            })?;

        // 6. Set permissions (Unix only)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(self.config.permissions);
            fs::set_permissions(&temp_path, perms).await.map_err(|e| {
                DeliveryError::Filesystem {
                    path: temp_path.clone(),
                    operation: "set_permissions".to_string(),
                    error: e.to_string(),
                }
            })?;
        }

        // 7. Atomic move to final location
        fs::rename(&temp_path, &file_path)
            .await
            .map_err(|e| DeliveryError::Filesystem {
                path: file_path.clone(),
                operation: "rename".to_string(),
                error: e.to_string(),
            })?;

        Ok(DeliveryResult::success(
            format!("filesystem:{}", file_path),
            start_time.elapsed(),
            formatted_output.len() as u64,
            Some(file_path),
        ))
    }

    fn validate_config(&self) -> Result<(), ValidationError> {
        // Validate path template
        if self.config.path_template.is_empty() {
            return Err(ValidationError::EmptyPath);
        }

        // Check template variables are valid
        self.template_engine
            .validate(&self.config.path_template)
            .map_err(|e| ValidationError::InvalidTemplate(e.to_string()))?;

        // Validate permissions (Unix only)
        #[cfg(unix)]
        if self.config.permissions > 0o777 {
            return Err(ValidationError::InvalidPermissions(self.config.permissions));
        }

        Ok(())
    }

    fn destination_type(&self) -> &'static str {
        "filesystem"
    }

    fn supports_retry(&self) -> bool {
        true
    }

    fn estimated_delivery_time(&self) -> std::time::Duration {
        std::time::Duration::from_millis(100) // Fast local filesystem operations
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use tempfile::TempDir;

    fn create_test_output() -> TaskOutput {
        TaskOutput {
            job_id: 123,
            task_id: 456,
            execution_id: 789,
            output_data: serde_json::json!({
                "result": "success",
                "data": {"temperature": 20.5, "humidity": 65}
            }),
            metadata: HashMap::new(),
            completed_at: chrono::Utc::now(),
            execution_duration: std::time::Duration::from_secs(5),
        }
    }

    fn create_test_context() -> DeliveryContext {
        let mut template_vars = HashMap::new();
        template_vars.insert("job_id".to_string(), "123".to_string());
        template_vars.insert("timestamp".to_string(), "20240106_143000".to_string());

        DeliveryContext {
            job_id: 123,
            task_name: "test-task".to_string(),
            task_version: "1.0.0".to_string(),
            timestamp: chrono::Utc::now(),
            environment: "test".to_string(),
            trace_id: "trace-123".to_string(),
            template_variables: template_vars,
        }
    }

    #[tokio::test]
    async fn test_filesystem_delivery_json() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("{{job_id}}_{{timestamp}}.json");

        let config = FilesystemConfig {
            path_template: file_path.to_string_lossy().to_string(),
            format: OutputFormat::Json,
            permissions: 0o644,
            create_dirs: true,
            overwrite: false,
            backup_existing: false,
        };

        let destination = FilesystemDestination::new(config, TemplateEngine::new());
        let output = create_test_output();
        let context = create_test_context();

        let result = destination.deliver(&output, &context).await.unwrap();

        assert!(result.success);
        assert!(result.size_bytes > 0);
        assert!(result.response_info.is_some());

        // Check file was created
        let expected_path = temp_dir.path().join("123_20240106_143000.json");
        assert!(expected_path.exists());

        // Check file content
        let content = fs::read_to_string(&expected_path).await.unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert_eq!(parsed["result"], "success");
    }

    #[tokio::test]
    async fn test_filesystem_delivery_csv() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("output.csv");

        let config = FilesystemConfig {
            path_template: file_path.to_string_lossy().to_string(),
            format: OutputFormat::Csv,
            permissions: 0o644,
            create_dirs: true,
            overwrite: true,
            backup_existing: false,
        };

        let destination = FilesystemDestination::new(config, TemplateEngine::new());

        // Create CSV-friendly output
        let mut output = create_test_output();
        output.output_data = serde_json::json!([
            {"name": "Alice", "age": 30, "city": "New York"},
            {"name": "Bob", "age": 25, "city": "San Francisco"}
        ]);

        let context = create_test_context();

        let result = destination.deliver(&output, &context).await.unwrap();

        assert!(result.success);
        assert!(file_path.exists());

        // Check CSV content
        let content = fs::read_to_string(&file_path).await.unwrap();

        // CSV headers might be in any order since HashMap keys are unordered
        // Just check that all expected headers and values are present
        assert!(content.contains("name"));
        assert!(content.contains("age"));
        assert!(content.contains("city"));
        assert!(content.contains("Alice"));
        assert!(content.contains("30"));
        assert!(content.contains("New York"));
        assert!(content.contains("Bob"));
        assert!(content.contains("25"));
        assert!(content.contains("San Francisco"));
    }

    #[tokio::test]
    async fn test_file_exists_error() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("existing.json");

        // Create existing file
        fs::write(&file_path, "existing content").await.unwrap();

        let config = FilesystemConfig {
            path_template: file_path.to_string_lossy().to_string(),
            format: OutputFormat::Json,
            permissions: 0o644,
            create_dirs: true,
            overwrite: false,
            backup_existing: false,
        };

        let destination = FilesystemDestination::new(config, TemplateEngine::new());
        let output = create_test_output();
        let context = create_test_context();

        let result = destination.deliver(&output, &context).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            DeliveryError::FileExists { .. } => {}
            _ => panic!("Expected FileExists error"),
        }
    }

    #[tokio::test]
    async fn test_backup_existing() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("backup_test.json");

        // Create existing file
        fs::write(&file_path, "original content").await.unwrap();

        let config = FilesystemConfig {
            path_template: file_path.to_string_lossy().to_string(),
            format: OutputFormat::Json,
            permissions: 0o644,
            create_dirs: true,
            overwrite: false,
            backup_existing: true,
        };

        let destination = FilesystemDestination::new(config, TemplateEngine::new());
        let output = create_test_output();
        let context = create_test_context();

        let result = destination.deliver(&output, &context).await.unwrap();

        assert!(result.success);
        assert!(file_path.exists());

        // Check backup file was created
        let backup_files: Vec<_> = std::fs::read_dir(temp_dir.path())
            .unwrap()
            .filter_map(|entry| {
                let path = entry.ok()?.path();
                let name = path.file_name()?.to_string_lossy();
                if name.starts_with("backup_test.json.backup.") {
                    Some(path)
                } else {
                    None
                }
            })
            .collect();

        assert_eq!(backup_files.len(), 1);

        // Check backup content
        let backup_content = fs::read_to_string(&backup_files[0]).await.unwrap();
        assert_eq!(backup_content, "original content");
    }

    #[test]
    fn test_validation() {
        let valid_config = FilesystemConfig {
            path_template: "/tmp/{{job_id}}.json".to_string(),
            format: OutputFormat::Json,
            permissions: 0o644,
            create_dirs: true,
            overwrite: false,
            backup_existing: false,
        };

        let destination = FilesystemDestination::new(valid_config, TemplateEngine::new());
        assert!(destination.validate_config().is_ok());

        let invalid_config = FilesystemConfig {
            path_template: "".to_string(), // Empty path
            format: OutputFormat::Json,
            permissions: 0o644,
            create_dirs: true,
            overwrite: false,
            backup_existing: false,
        };

        let destination = FilesystemDestination::new(invalid_config, TemplateEngine::new());
        assert!(destination.validate_config().is_err());
    }
}
