//! Filesystem output destination implementation

use async_trait::async_trait;
use std::path::{Path, PathBuf};
use std::time::Instant;
use tokio::fs;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use crate::{
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
    
    /// Normalize path for cross-platform compatibility
    fn normalize_path(path: &str) -> PathBuf {
        // Replace forward slashes with platform-specific separators
        let normalized = if cfg!(windows) {
            path.replace('/', std::path::MAIN_SEPARATOR_STR)
        } else {
            path.to_string()
        };
        
        PathBuf::from(normalized)
    }
    
    /// Validate path for cross-platform compatibility
    fn validate_path(path: &str) -> Result<(), DeliveryError> {
        // Check for invalid characters based on platform
        if cfg!(windows) {
            // Windows forbidden characters: < > : " | ? * 
            let forbidden_chars = ['<', '>', ':', '"', '|', '?', '*'];
            if path.chars().any(|c| forbidden_chars.contains(&c)) {
                return Err(DeliveryError::Filesystem {
                    path: path.to_string(),
                    operation: "validate".to_string(),
                    error: "Path contains invalid characters for Windows".to_string(),
                });
            }
            
            // Check for reserved names (CON, PRN, AUX, etc.)
            let path_obj = Path::new(path);
            if let Some(filename) = path_obj.file_name().and_then(|n| n.to_str()) {
                let reserved_names = [
                    "CON", "PRN", "AUX", "NUL",
                    "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8", "COM9",
                    "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9"
                ];
                
                let filename_upper = filename.to_uppercase();
                let base_name = filename_upper.split('.').next().unwrap_or("");
                
                if reserved_names.contains(&base_name) {
                    return Err(DeliveryError::Filesystem {
                        path: path.to_string(),
                        operation: "validate".to_string(),
                        error: format!("Filename '{}' is reserved on Windows", filename),
                    });
                }
            }
        }
        
        // Check for null bytes (invalid on all platforms)
        if path.contains('\0') {
            return Err(DeliveryError::Filesystem {
                path: path.to_string(),
                operation: "validate".to_string(),
                error: "Path contains null bytes".to_string(),
            });
        }
        
        Ok(())
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
            #[cfg(feature = "yaml")]
            OutputFormat::Yaml => {
                serde_yaml::to_string(data)
                    .map(|s| s.into_bytes())
                    .map_err(|e| DeliveryError::Serialization {
                        format: "yaml".to_string(),
                        error: e.to_string(),
                    })
            }
            #[cfg(not(feature = "yaml"))]
            OutputFormat::Yaml => Err(DeliveryError::Serialization {
                format: "yaml".to_string(),
                error: "YAML support not enabled".to_string(),
            }),
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
                    .render_json(template, data)
                    .map_err(|e| match e {
                        DeliveryError::TemplateRender { template: t, error: err } => {
                            DeliveryError::TemplateRender { template: t, error: err }
                        }
                        _ => DeliveryError::TemplateRender {
                            template: template.clone(),
                            error: e.to_string(),
                        }
                    })?;
                Ok(rendered.into_bytes())
            }
            #[cfg(feature = "csv")]
            OutputFormat::Csv => self.convert_to_csv(data),
            #[cfg(not(feature = "csv"))]
            OutputFormat::Csv => Err(DeliveryError::Serialization {
                format: "csv".to_string(),
                error: "CSV support not enabled".to_string(),
            }),
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

    /// Create backup of existing file
    async fn backup_file(&self, path: &Path) -> Result<(), DeliveryError> {
        if !path.exists() {
            return Ok(());
        }

        let backup_path = path.with_extension(
            format!("{}.backup", path.extension().and_then(|s| s.to_str()).unwrap_or(""))
        );

        fs::copy(path, backup_path).await
            .map_err(|e| DeliveryError::Filesystem {
                path: path.to_string_lossy().to_string(),
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

        // Render the path template
        let rendered_path = self
            .template_engine
            .render(&self.config.path_template, &context.template_variables)?;

        // Validate and normalize the path for cross-platform compatibility
        Self::validate_path(&rendered_path)?;
        let normalized_path = Self::normalize_path(&rendered_path);
        let path = normalized_path.as_path();

        // Check if file exists and handle overwrite policy
        if path.exists() && !self.config.overwrite {
            return Err(DeliveryError::FileExists {
                path: rendered_path,
            });
        }

        // Create backup if requested and file exists
        if self.config.backup_existing && path.exists() {
            self.backup_file(path).await?;
        }

        // Create parent directories if needed
        if self.config.create_dirs {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).await
                    .map_err(|e| DeliveryError::Filesystem {
                        path: parent.to_string_lossy().to_string(),
                        operation: "create_dirs".to_string(),
                        error: e.to_string(),
                    })?;
            }
        }

        // Format the output data
        let formatted_data = self.format_output(&output.output_data)?;
        let size_bytes = formatted_data.len() as u64;

        // Write the file
        fs::write(path, &formatted_data).await
            .map_err(|e| DeliveryError::Filesystem {
                path: rendered_path.clone(),
                operation: "write".to_string(),
                error: e.to_string(),
            })?;

        // Set file permissions (Unix only)
        #[cfg(unix)]
        if self.config.permissions != 0 {
            let permissions = std::fs::Permissions::from_mode(self.config.permissions);
            std::fs::set_permissions(path, permissions)
                .map_err(|e| DeliveryError::Filesystem {
                    path: rendered_path.clone(),
                    operation: "set_permissions".to_string(),
                    error: e.to_string(),
                })?;
        }
        
        // On Windows, permissions are handled differently through ACLs
        // For now, we just ensure the file is writable
        #[cfg(windows)]
        if self.config.permissions != 0 {
            // On Windows, we can only set read-only flag
            let mut perms = fs::metadata(path).await
                .map_err(|e| DeliveryError::Filesystem {
                    path: rendered_path.clone(),
                    operation: "get_metadata".to_string(),
                    error: e.to_string(),
                })?
                .permissions();
            
            // If permissions don't include write bit (owner write = 0o200), set read-only
            let readonly = (self.config.permissions & 0o200) == 0;
            perms.set_readonly(readonly);
            
            fs::set_permissions(path, perms).await
                .map_err(|e| DeliveryError::Filesystem {
                    path: rendered_path.clone(),
                    operation: "set_permissions".to_string(),
                    error: e.to_string(),
                })?;
        }

        let delivery_time = start_time.elapsed();

        Ok(DeliveryResult::success(
            "filesystem".to_string(),
            delivery_time,
            size_bytes,
            Some(rendered_path),
        ))
    }

    fn validate_config(&self) -> Result<(), ValidationError> {
        if self.config.path_template.is_empty() {
            return Err(ValidationError::EmptyPath);
        }

        // Validate template syntax
        self.template_engine
            .validate(&self.config.path_template)
            .map_err(|e| ValidationError::InvalidTemplate(e.to_string()))?;

        // Validate permissions (should be valid octal on Unix, ignored on Windows)
        #[cfg(unix)]
        if self.config.permissions > 0o777 {
            return Err(ValidationError::InvalidPermissions(self.config.permissions));
        }
        
        // On Windows, only validate that permissions are reasonable (not used for ACLs)
        #[cfg(windows)]
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
        std::time::Duration::from_millis(100) // File I/O is usually fast
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::template::TemplateEngine;
    use crate::OutputFormat;

    #[test]
    fn test_cross_platform_path_normalization() {
        // Test Unix-style paths
        let unix_path = "/results/2024/01/06/output.json";
        let normalized = FilesystemDestination::normalize_path(unix_path);
        
        if cfg!(windows) {
            assert_eq!(normalized.to_string_lossy(), "\\results\\2024\\01\\06\\output.json");
        } else {
            assert_eq!(normalized.to_string_lossy(), unix_path);
        }
    }

    #[test]
    fn test_path_validation_windows() {
        if cfg!(windows) {
            // Test invalid characters
            assert!(FilesystemDestination::validate_path("output<test>.json").is_err());
            assert!(FilesystemDestination::validate_path("output|test.json").is_err());
            assert!(FilesystemDestination::validate_path("output\"test.json").is_err());
            
            // Test reserved names
            assert!(FilesystemDestination::validate_path("CON.txt").is_err());
            assert!(FilesystemDestination::validate_path("PRN.json").is_err());
            assert!(FilesystemDestination::validate_path("AUX").is_err());
            assert!(FilesystemDestination::validate_path("NUL.log").is_err());
            
            // Test valid paths
            assert!(FilesystemDestination::validate_path("output.json").is_ok());
            assert!(FilesystemDestination::validate_path("results/2024/output.json").is_ok());
        }
    }

    #[test]
    fn test_path_validation_universal() {
        // Test null bytes (invalid on all platforms)
        assert!(FilesystemDestination::validate_path("output\0test.json").is_err());
        
        // Test valid paths
        assert!(FilesystemDestination::validate_path("output.json").is_ok());
        assert!(FilesystemDestination::validate_path("results/sub/output.json").is_ok());
        assert!(FilesystemDestination::validate_path("./relative/path.json").is_ok());
    }

    #[test]
    fn test_filesystem_config_validation() {
        let template_engine = TemplateEngine::new();
        
        // Test valid config
        let valid_config = FilesystemConfig {
            path_template: "/results/{{job_id}}.json".to_string(),
            format: OutputFormat::Json,
            permissions: 0o644,
            create_dirs: true,
            overwrite: false,
            backup_existing: false,
        };
        
        let destination = FilesystemDestination::new(valid_config, template_engine.clone());
        assert!(destination.validate_config().is_ok());
        
        // Test invalid permissions
        let invalid_config = FilesystemConfig {
            path_template: "/results/{{job_id}}.json".to_string(),
            format: OutputFormat::Json,
            permissions: 999, // Invalid permissions (not octal)
            create_dirs: true,
            overwrite: false,
            backup_existing: false,
        };
        
        let destination = FilesystemDestination::new(invalid_config, template_engine.clone());
        assert!(destination.validate_config().is_err());
        
        // Test empty path
        let empty_path_config = FilesystemConfig {
            path_template: "".to_string(),
            format: OutputFormat::Json,
            permissions: 0o644,
            create_dirs: true,
            overwrite: false,
            backup_existing: false,
        };
        
        let destination = FilesystemDestination::new(empty_path_config, template_engine);
        assert!(destination.validate_config().is_err());
    }

    #[tokio::test]
    async fn test_cross_platform_output_formats() {
        use serde_json::json;
        
        let template_engine = TemplateEngine::new();
        let config = FilesystemConfig {
            path_template: "test_output.json".to_string(),
            format: OutputFormat::Json,
            permissions: 0o644,
            create_dirs: false,
            overwrite: true,
            backup_existing: false,
        };
        
        let destination = FilesystemDestination::new(config, template_engine);
        
        // Test format_output with different data types
        let test_data = json!({
            "status": "success",
            "result": 42,
            "message": "Cross-platform test"
        });
        
        let formatted = destination.format_output(&test_data).unwrap();
        assert!(!formatted.is_empty());
        
        // Verify it's valid JSON
        let parsed: serde_json::Value = serde_json::from_slice(&formatted).unwrap();
        assert_eq!(parsed["status"], "success");
        assert_eq!(parsed["result"], 42);
    }
}