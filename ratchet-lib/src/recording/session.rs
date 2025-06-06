use anyhow::Result;
use chrono::{DateTime, Utc};
use serde_json::{json, Value as JsonValue};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tracing::{debug, info};

// Global recording state
lazy_static::lazy_static! {
    static ref RECORDING_STATE: Arc<Mutex<Option<RecordingState>>> = Arc::new(Mutex::new(None));
}

#[derive(Debug, Clone)]
struct RecordingState {
    session_dir: PathBuf,
    entries: Vec<JsonValue>,
}

/// Set the recording directory for the current session
pub fn set_recording_dir(session_dir: PathBuf) -> Result<()> {
    debug!("Setting recording directory: {:?}", session_dir);

    let mut state = RECORDING_STATE.lock().unwrap();
    *state = Some(RecordingState {
        session_dir,
        entries: Vec::new(),
    });

    Ok(())
}

/// Record an HTTP request/response pair in HAR format
pub fn record_http_request(
    url: &str,
    method: &str,
    request_headers: Option<&HashMap<String, String>>,
    request_body: Option<&str>,
    response_status: u16,
    response_headers: Option<&HashMap<String, String>>,
    response_body: &str,
    started_at: DateTime<Utc>,
    duration_ms: u64,
) -> Result<()> {
    let mut state = RECORDING_STATE.lock().unwrap();

    if let Some(ref mut recording_state) = state.as_mut() {
        debug!("Recording HTTP request: {} {}", method, url);

        // Build request headers
        let req_headers: Vec<JsonValue> = if let Some(headers) = request_headers {
            headers
                .iter()
                .map(|(name, value)| {
                    json!({
                        "name": name,
                        "value": value,
                        "comment": ""
                    })
                })
                .collect()
        } else {
            Vec::new()
        };

        // Build response headers
        let resp_headers: Vec<JsonValue> = if let Some(headers) = response_headers {
            headers
                .iter()
                .map(|(name, value)| {
                    json!({
                        "name": name,
                        "value": value,
                        "comment": ""
                    })
                })
                .collect()
        } else {
            Vec::new()
        };

        // Build HAR entry
        let entry = json!({
            "startedDateTime": started_at.to_rfc3339(),
            "time": duration_ms,
            "request": {
                "method": method,
                "url": url,
                "httpVersion": "HTTP/1.1",
                "cookies": [],
                "headers": req_headers,
                "queryString": [],
                "postData": request_body.map(|body| json!({
                    "mimeType": "application/x-www-form-urlencoded",
                    "text": body,
                    "params": []
                })),
                "headersSize": -1,
                "bodySize": request_body.map_or(0, |b| b.len())
            },
            "response": {
                "status": response_status,
                "statusText": match response_status {
                    200 => "OK",
                    400 => "Bad Request",
                    401 => "Unauthorized",
                    404 => "Not Found",
                    500 => "Internal Server Error",
                    _ => "Unknown"
                },
                "httpVersion": "HTTP/1.1",
                "cookies": [],
                "headers": resp_headers,
                "content": {
                    "size": response_body.len(),
                    "mimeType": "application/json",
                    "text": response_body,
                    "compression": 0
                },
                "redirectURL": "",
                "headersSize": -1,
                "bodySize": response_body.len()
            },
            "cache": {},
            "timings": {
                "blocked": 0,
                "dns": 0,
                "connect": 0,
                "send": 0,
                "wait": duration_ms,
                "receive": 0,
                "ssl": -1
            },
            "serverIPAddress": "",
            "connection": "",
            "comment": ""
        });

        recording_state.entries.push(entry);
        info!(
            "Recorded HTTP request {} {} -> {}",
            method, url, response_status
        );
    }

    Ok(())
}

/// Finalize and save the HAR file
pub fn finalize_recording() -> Result<()> {
    let mut state = RECORDING_STATE.lock().unwrap();

    if let Some(recording_state) = state.take() {
        debug!(
            "Finalizing recording with {} entries",
            recording_state.entries.len()
        );

        let har = json!({
            "log": {
                "version": "1.2",
                "creator": {
                    "name": "Ratchet",
                    "version": "0.1.0",
                    "comment": ""
                },
                "browser": {
                    "name": "Ratchet",
                    "version": "0.1.0",
                    "comment": ""
                },
                "pages": [],
                "entries": recording_state.entries,
                "comment": ""
            }
        });

        let har_file = recording_state.session_dir.join("requests.har");
        let har_json = serde_json::to_string_pretty(&har)?;
        fs::write(&har_file, har_json)?;

        info!("Saved HAR file: {:?}", har_file);
    }

    Ok(())
}

/// Check if recording is currently active
pub fn is_recording() -> bool {
    let state = RECORDING_STATE.lock().unwrap();
    state.is_some()
}

/// Get the current recording directory
pub fn get_recording_dir() -> Option<PathBuf> {
    let state = RECORDING_STATE.lock().unwrap();
    state.as_ref().map(|s| s.session_dir.clone())
}

/// Record task input JSON
pub fn record_input(input_json: &JsonValue) -> Result<()> {
    let state = RECORDING_STATE.lock().unwrap();

    if let Some(recording_state) = state.as_ref() {
        debug!("Recording task input JSON");

        let input_file = recording_state.session_dir.join("input.json");
        let input_pretty = serde_json::to_string_pretty(input_json)?;
        fs::write(&input_file, input_pretty)?;

        info!("Saved input JSON: {:?}", input_file);
    }

    Ok(())
}

/// Record task output JSON
pub fn record_output(output_json: &JsonValue) -> Result<()> {
    let state = RECORDING_STATE.lock().unwrap();

    if let Some(recording_state) = state.as_ref() {
        debug!("Recording task output JSON");

        let output_file = recording_state.session_dir.join("output.json");
        let output_pretty = serde_json::to_string_pretty(output_json)?;
        fs::write(&output_file, output_pretty)?;

        info!("Saved output JSON: {:?}", output_file);
    }

    Ok(())
}
