//! HTTP recording compatibility layer for CLI tools
//! 
//! This module provides a compatibility layer for CLI tools to use HTTP recording
//! functionality from both ratchet_lib and ratchet-http.

use anyhow::Result;
use std::path::PathBuf;
use tracing::{debug, info};

#[cfg(feature = "recording")]
use ratchet_http::recording as http_recording;

/// Set the recording directory for HTTP requests
/// 
/// This function provides a unified API for setting the recording directory
/// that works with both legacy ratchet_lib and modern ratchet-http.
pub fn set_recording_dir(session_dir: PathBuf) -> Result<()> {
    info!("Setting recording directory: {:?}", session_dir);
    
    #[cfg(feature = "recording")]
    {
        // Use modern ratchet-http recording
        http_recording::set_recording_dir(session_dir.clone())?;
        debug!("Set recording directory using ratchet-http");
    }
    
    // Also set legacy recording if available
    #[cfg(feature = "legacy")]
    {
        ratchet_lib::recording::set_recording_dir(session_dir)?;
        debug!("Set recording directory using ratchet_lib (legacy)");
    }
    
    #[cfg(not(any(feature = "recording", feature = "legacy")))]
    {
        debug!("Recording not available - no recording features enabled");
    }
    
    Ok(())
}

/// Get the current recording directory
#[cfg(feature = "recording")]
pub fn get_recording_dir() -> Option<PathBuf> {
    http_recording::get_recording_dir()
}

#[cfg(not(feature = "recording"))]
pub fn get_recording_dir() -> Option<PathBuf> {
    None
}

/// Check if recording is currently active
#[cfg(feature = "recording")]
pub fn is_recording() -> bool {
    http_recording::is_recording()
}

#[cfg(not(feature = "recording"))]
pub fn is_recording() -> bool {
    false
}

/// Finalize recording and write HAR file
#[cfg(feature = "recording")]
pub fn finalize_recording() -> Result<()> {
    debug!("Finalizing HTTP recording");
    http_recording::finalize_recording()
}

#[cfg(not(feature = "recording"))]
pub fn finalize_recording() -> Result<()> {
    debug!("Recording finalization not available - recording not enabled");
    Ok(())
}