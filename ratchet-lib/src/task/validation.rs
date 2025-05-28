use super::{Task, TaskError};
use boa_engine::{Context as BoaContext, Source};
use tracing::{debug, info, warn};

/// Validate that the task is properly structured and syntactically correct
pub fn validate_task(task: &mut Task) -> Result<(), TaskError> {
    debug!("Validating task: {} ({})", task.metadata.label, task.metadata.uuid);
    
    // 1. Validate input schema is valid JSON Schema
    debug!("Validating input schema");
    if !task.input_schema.is_object() {
        warn!("Input schema is not a valid JSON object");
        return Err(TaskError::InvalidJsonSchema(
            "Input schema must be a valid JSON object".to_string()
        ));
    }
    
    // 2. Validate output schema is valid JSON Schema
    debug!("Validating output schema");
    if !task.output_schema.is_object() {
        warn!("Output schema is not a valid JSON object");
        return Err(TaskError::InvalidJsonSchema(
            "Output schema must be a valid JSON object".to_string()
        ));
    }
    
    // 3. Validate that the JavaScript code can be parsed
    debug!("Validating JavaScript content");
    task.ensure_content_loaded()?;
    let js_content = task.get_js_content()?;
    
    // We'll use a basic heuristic first - check if it contains a function definition
    if !js_content.contains("function") {
        warn!("JavaScript code does not contain a function definition");
        return Err(TaskError::JavaScriptParseError(
            "JavaScript code does not contain a function definition".to_string()
        ));
    }
    
    // 4. Try to parse the JavaScript code using BoaJS
    // This will catch syntax errors in the JavaScript code
    debug!("Parsing JavaScript with BoaJS engine");
    let mut context = BoaContext::default();
    let result = context.eval(Source::from_bytes(js_content.as_ref()));
    if result.is_err() {
        let error = result.err().unwrap();
        warn!("JavaScript syntax error: {}", error);
        return Err(TaskError::JavaScriptParseError(
            format!("JavaScript syntax error: {}", error)
        ));
    }
    
    // 5. Validate that the code returns a function or is a callable object
    let js_result = result.unwrap();
    if !js_result.is_callable() && !js_result.is_object() {
        warn!("JavaScript code does not return a callable function or object");
        return Err(TaskError::JavaScriptParseError(
            "JavaScript code must return a callable function or object".to_string()
        ));
    }
    
    // All validations passed
    info!("Task validation completed successfully: {} ({})", task.metadata.label, task.metadata.uuid);
    Ok(())
}