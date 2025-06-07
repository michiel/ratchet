use super::enhanced::{Task, TaskBuilder, TaskMetadata, TaskType};
use super::TaskError;
use serde_json::Value as JsonValue;
use std::fs;
use std::io;
use std::path::Path;
use std::sync::Arc;
use tempfile::TempDir;
use tracing::{debug, info, warn};
#[cfg(feature = "output")]
use zip::ZipArchive;

/// Load a task from the filesystem or a ZIP file
pub fn load_from_fs(path: impl AsRef<Path>) -> Result<Task, TaskError> {
    let path = path.as_ref();

    debug!("Loading task from path: {:?}", path);

    // Check if path exists
    if !path.exists() {
        warn!("Task path does not exist: {:?}", path);
        return Err(TaskError::TaskFileNotFound(
            path.to_string_lossy().to_string(),
        ));
    }

    // Check if the path is a file (potentially a ZIP) or a directory
    if path.is_file() {
        // Check if it's a ZIP file
        let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if extension.to_lowercase() == "zip" {
            debug!("Loading task from ZIP file: {:?}", path);
            return load_from_zip(path);
        } else {
            warn!("File is not a ZIP file: {:?}", path);
            return Err(TaskError::InvalidTaskStructure(format!(
                "File {} is not a ZIP file",
                path.to_string_lossy()
            )));
        }
    } else if !path.is_dir() {
        warn!("Path is neither a directory nor a ZIP file: {:?}", path);
        return Err(TaskError::InvalidTaskStructure(format!(
            "Path {} is neither a directory nor a ZIP file",
            path.to_string_lossy()
        )));
    }

    // Path is a directory, process it directly
    debug!("Loading task from directory: {:?}", path);
    load_from_directory(path)
}

/// Load a task from a directory
pub fn load_from_directory(path: &Path) -> Result<Task, TaskError> {
    debug!("Loading task from directory: {:?}", path);

    // Read metadata.json
    let metadata_path = path.join("metadata.json");
    if !metadata_path.exists() {
        warn!("Metadata file not found: {:?}", metadata_path);
        return Err(TaskError::TaskFileNotFound(format!(
            "Metadata file not found at {}",
            metadata_path.to_string_lossy()
        )));
    }

    let metadata_json = fs::read_to_string(&metadata_path)?;
    
    // Parse the legacy metadata format
    #[derive(serde::Deserialize)]
    struct LegacyMetadata {
        uuid: uuid::Uuid,
        version: String,
        label: String,
        description: String,
    }
    
    let legacy_metadata: LegacyMetadata = serde_json::from_str(&metadata_json)?;
    let metadata = TaskMetadata::new(
        legacy_metadata.uuid,
        legacy_metadata.version,
        legacy_metadata.label,
        legacy_metadata.description,
    );

    debug!(
        "Task metadata loaded: {} ({})",
        metadata.label, metadata.uuid
    );

    // Read input schema
    let input_schema_path = path.join("input.schema.json");
    if !input_schema_path.exists() {
        warn!("Input schema file not found: {:?}", input_schema_path);
        return Err(TaskError::TaskFileNotFound(format!(
            "Input schema file not found at {}",
            input_schema_path.to_string_lossy()
        )));
    }

    let input_schema_json = fs::read_to_string(&input_schema_path)?;
    let input_schema: JsonValue = serde_json::from_str(&input_schema_json)?;

    // Read output schema
    let output_schema_path = path.join("output.schema.json");
    if !output_schema_path.exists() {
        warn!("Output schema file not found: {:?}", output_schema_path);
        return Err(TaskError::TaskFileNotFound(format!(
            "Output schema file not found at {}",
            output_schema_path.to_string_lossy()
        )));
    }

    let output_schema_json = fs::read_to_string(&output_schema_path)?;
    let output_schema: JsonValue = serde_json::from_str(&output_schema_json)?;

    // Check for JS file
    let js_file_path = path.join("main.js");
    if !js_file_path.exists() {
        warn!("JavaScript file not found: {:?}", js_file_path);
        return Err(TaskError::TaskFileNotFound(format!(
            "JavaScript file not found at {}",
            js_file_path.to_string_lossy()
        )));
    }

    // Create the task type (without loading content initially)
    let task_type = TaskType::JsTask {
        path: js_file_path.to_string_lossy().to_string(),
        content: None, // Content is loaded lazily
    };

    info!(
        "Successfully loaded task: {} ({})",
        metadata.label, metadata.uuid
    );

    // Use TaskBuilder to construct the enhanced task
    TaskBuilder::new()
        .with_metadata(metadata)
        .with_task_type(task_type)
        .with_input_schema(input_schema)
        .with_output_schema(output_schema)
        .with_path(path.to_path_buf())
        .build()
}

/// Load a task from a ZIP file
pub fn load_from_zip(zip_path: &Path) -> Result<Task, TaskError> {
    debug!("Loading task from ZIP file: {:?}", zip_path);

    // Create a temporary directory to extract the ZIP
    let temp_dir = TempDir::new()?;
    let temp_dir_arc = Arc::new(temp_dir);
    let extract_path = temp_dir_arc.path();

    debug!("Created temporary directory: {:?}", extract_path);

    // Open the ZIP file
    let zip_file = fs::File::open(zip_path)?;
    let mut archive = ZipArchive::new(zip_file)?;

    // Extract all files from the ZIP to the temporary directory
    debug!("Extracting {} files from ZIP", archive.len());
    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let file_path = match file.enclosed_name() {
            Some(path) => path.to_owned(),
            None => {
                warn!("Skipping file with unsafe name at index {}", i);
                continue; // Skip files with unsafe names
            }
        };

        let output_path = extract_path.join(&file_path);

        // Create directory structure if needed
        if file.is_dir() {
            fs::create_dir_all(&output_path)?;
        } else {
            if let Some(parent) = output_path.parent() {
                if !parent.exists() {
                    fs::create_dir_all(parent)?;
                }
            }
            let mut output_file = fs::File::create(&output_path)?;
            io::copy(&mut file, &mut output_file)?;
        }
    }

    // Determine the root directory of the task within the extracted ZIP
    // We look for a directory that contains metadata.json
    let root_dir = if extract_path.join("metadata.json").exists() {
        extract_path.to_path_buf()
    } else {
        // Try to find a subdirectory with metadata.json
        let entries = fs::read_dir(extract_path)?;
        let mut task_dir = None;

        for entry in entries {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() && path.join("metadata.json").exists() {
                task_dir = Some(path);
                break;
            }
        }

        task_dir.ok_or_else(|| {
            TaskError::TaskFileNotFound(format!(
                "Could not find metadata.json in ZIP file {}",
                zip_path.to_string_lossy()
            ))
        })?
    };

    // Now load the task from the extracted directory
    debug!("Loading task from extracted directory: {:?}", root_dir);
    let mut task = load_from_directory(&root_dir)?;

    // Store the temp_dir in the task to keep it alive as long as the task exists
    task._temp_dir = Some(temp_dir_arc);

    info!(
        "Successfully loaded task from ZIP: {} ({})",
        task.metadata.label, task.metadata.uuid
    );

    Ok(task)
}
