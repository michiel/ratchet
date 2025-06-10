//! Filesystem output destination implementation

use async_trait::async_trait;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::time::Instant;
use tokio::fs;

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

        let path = Path::new(&rendered_path);

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

        // Validate permissions (should be valid octal)
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