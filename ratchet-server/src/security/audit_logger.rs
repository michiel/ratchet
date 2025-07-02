//! Audit logging for security events and compliance
//!
//! This module provides comprehensive audit logging capabilities for security
//! events, user actions, and system operations.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncWriteExt, BufWriter};
use tokio::sync::{mpsc, RwLock};
use tracing::{error, info, warn};

use crate::config::{AuditConfig, AuditExportFormat, AuditLogLevel};
use super::{SecurityEvent, SecurityContext, SecurityEventType, SecurityEventSeverity};

/// Audit log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogEntry {
    /// Entry ID
    pub id: String,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Security event
    pub event: SecurityEvent,
    /// Additional metadata
    pub metadata: AuditMetadata,
}

/// Audit metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditMetadata {
    /// Source component
    pub source: String,
    /// Log level
    pub level: AuditLogLevel,
    /// Environment information
    pub environment: HashMap<String, String>,
    /// Request tracing information
    pub trace_id: Option<String>,
    /// Span ID for distributed tracing
    pub span_id: Option<String>,
}

impl Default for AuditMetadata {
    fn default() -> Self {
        Self {
            source: "ratchet-server".to_string(),
            level: AuditLogLevel::Info,
            environment: HashMap::new(),
            trace_id: None,
            span_id: None,
        }
    }
}

/// Audit log storage backend trait
#[async_trait::async_trait]
pub trait AuditStorage: Send + Sync {
    /// Store an audit log entry
    async fn store(&self, entry: &AuditLogEntry) -> Result<()>;
    
    /// Query audit log entries
    async fn query(&self, query: &AuditQuery) -> Result<Vec<AuditLogEntry>>;
    
    /// Export audit logs
    async fn export(&self, format: AuditExportFormat, query: &AuditQuery) -> Result<Vec<u8>>;
    
    /// Cleanup old audit logs
    async fn cleanup(&self, retention_days: u32) -> Result<u64>;
}

/// Audit query parameters
#[derive(Debug, Clone)]
pub struct AuditQuery {
    /// Start date filter
    pub start_date: Option<DateTime<Utc>>,
    /// End date filter
    pub end_date: Option<DateTime<Utc>>,
    /// Event type filter
    pub event_types: Vec<SecurityEventType>,
    /// Severity filter
    pub severities: Vec<SecurityEventSeverity>,
    /// User ID filter
    pub user_ids: Vec<String>,
    /// Repository ID filter
    pub repository_ids: Vec<i32>,
    /// Limit results
    pub limit: Option<usize>,
    /// Offset for pagination
    pub offset: Option<usize>,
}

impl Default for AuditQuery {
    fn default() -> Self {
        Self {
            start_date: None,
            end_date: None,
            event_types: Vec::new(),
            severities: Vec::new(),
            user_ids: Vec::new(),
            repository_ids: Vec::new(),
            limit: None,
            offset: None,
        }
    }
}

/// File-based audit storage implementation
pub struct FileAuditStorage {
    /// Base directory for audit logs
    base_path: PathBuf,
    /// File writer
    writer: Arc<RwLock<Option<BufWriter<File>>>>,
    /// Current log file path
    current_file: Arc<RwLock<Option<PathBuf>>>,
}

impl FileAuditStorage {
    /// Create a new file audit storage
    pub async fn new(base_path: PathBuf) -> Result<Self> {
        // Ensure directory exists
        tokio::fs::create_dir_all(&base_path).await
            .context("Failed to create audit log directory")?;

        let storage = Self {
            base_path,
            writer: Arc::new(RwLock::new(None)),
            current_file: Arc::new(RwLock::new(None)),
        };

        storage.rotate_log_file().await?;
        Ok(storage)
    }

    /// Rotate to a new log file (daily rotation)
    async fn rotate_log_file(&self) -> Result<()> {
        let date = Utc::now().format("%Y-%m-%d");
        let file_path = self.base_path.join(format!("audit-{}.jsonl", date));

        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&file_path)
            .await
            .context("Failed to open audit log file")?;

        let writer = BufWriter::new(file);

        {
            let mut current_writer = self.writer.write().await;
            if let Some(mut old_writer) = current_writer.take() {
                old_writer.flush().await.ok();
            }
            *current_writer = Some(writer);
        }

        {
            let mut current_file = self.current_file.write().await;
            *current_file = Some(file_path);
        }

        Ok(())
    }

    /// Check if log rotation is needed
    async fn should_rotate(&self) -> bool {
        let current_file = self.current_file.read().await;
        if let Some(file_path) = current_file.as_ref() {
            if let Some(file_name) = file_path.file_stem().and_then(|s| s.to_str()) {
                let today = Utc::now().format("%Y-%m-%d").to_string();
                let expected_name = format!("audit-{}", today);
                return !file_name.starts_with(&expected_name);
            }
        }
        true
    }
}

#[async_trait::async_trait]
impl AuditStorage for FileAuditStorage {
    async fn store(&self, entry: &AuditLogEntry) -> Result<()> {
        // Check if rotation is needed
        if self.should_rotate().await {
            self.rotate_log_file().await?;
        }

        let json_line = serde_json::to_string(entry)
            .context("Failed to serialize audit log entry")?;

        let mut writer_guard = self.writer.write().await;
        if let Some(writer) = writer_guard.as_mut() {
            writer.write_all(json_line.as_bytes()).await
                .context("Failed to write audit log entry")?;
            writer.write_all(b"\n").await
                .context("Failed to write newline")?;
            writer.flush().await
                .context("Failed to flush audit log writer")?;
        }

        Ok(())
    }

    async fn query(&self, query: &AuditQuery) -> Result<Vec<AuditLogEntry>> {
        let mut entries = Vec::new();
        let mut dir_entries = tokio::fs::read_dir(&self.base_path).await
            .context("Failed to read audit log directory")?;

        while let Some(entry) = dir_entries.next_entry().await? {
            let file_path = entry.path();
            if file_path.extension().and_then(|s| s.to_str()) == Some("jsonl") {
                let content = tokio::fs::read_to_string(&file_path).await
                    .context("Failed to read audit log file")?;

                for line in content.lines() {
                    if line.trim().is_empty() {
                        continue;
                    }

                    match serde_json::from_str::<AuditLogEntry>(line) {
                        Ok(entry) => {
                            if self.matches_query(&entry, query) {
                                entries.push(entry);
                            }
                        }
                        Err(e) => {
                            warn!("Failed to parse audit log entry: {}", e);
                        }
                    }
                }
            }
        }

        // Sort by timestamp (newest first)
        entries.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        // Apply limit and offset
        if let Some(offset) = query.offset {
            if offset < entries.len() {
                entries = entries.into_iter().skip(offset).collect();
            } else {
                entries.clear();
            }
        }

        if let Some(limit) = query.limit {
            entries.truncate(limit);
        }

        Ok(entries)
    }

    async fn export(&self, format: AuditExportFormat, query: &AuditQuery) -> Result<Vec<u8>> {
        let entries = self.query(query).await?;

        match format {
            AuditExportFormat::JSON => {
                let json = serde_json::to_vec_pretty(&entries)
                    .context("Failed to serialize audit entries to JSON")?;
                Ok(json)
            }
            AuditExportFormat::CSV => {
                let mut csv_data = String::new();
                csv_data.push_str("timestamp,event_type,severity,message,user_id,repository_id,correlation_id\n");

                for entry in entries {
                    csv_data.push_str(&format!(
                        "{},{:?},{:?},{},{},{},{}\n",
                        entry.timestamp.format("%Y-%m-%d %H:%M:%S UTC"),
                        entry.event.event_type,
                        entry.event.severity,
                        entry.event.message.replace(",", ";"),
                        entry.event.context.user_id.unwrap_or_default(),
                        entry.event.repository_id.map(|id| id.to_string()).unwrap_or_default(),
                        entry.event.context.correlation_id
                    ));
                }

                Ok(csv_data.into_bytes())
            }
            AuditExportFormat::XML => {
                let mut xml_data = String::new();
                xml_data.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<audit_logs>\n");

                for entry in entries {
                    xml_data.push_str(&format!(
                        "  <entry id=\"{}\" timestamp=\"{}\">\n    <event type=\"{:?}\" severity=\"{:?}\">{}</event>\n    <user_id>{}</user_id>\n    <repository_id>{}</repository_id>\n    <correlation_id>{}</correlation_id>\n  </entry>\n",
                        entry.id,
                        entry.timestamp.format("%Y-%m-%d %H:%M:%S UTC"),
                        entry.event.event_type,
                        entry.event.severity,
                        entry.event.message,
                        entry.event.context.user_id.unwrap_or_default(),
                        entry.event.repository_id.map(|id| id.to_string()).unwrap_or_default(),
                        entry.event.context.correlation_id
                    ));
                }

                xml_data.push_str("</audit_logs>\n");
                Ok(xml_data.into_bytes())
            }
            AuditExportFormat::Parquet => {
                // For simplicity, return JSON format for Parquet requests
                // In a real implementation, you would use a Parquet library
                self.export(AuditExportFormat::JSON, query).await
            }
        }
    }

    async fn cleanup(&self, retention_days: u32) -> Result<u64> {
        let cutoff_date = Utc::now() - chrono::Duration::days(retention_days as i64);
        let mut deleted_count = 0;

        let mut dir_entries = tokio::fs::read_dir(&self.base_path).await
            .context("Failed to read audit log directory")?;

        while let Some(entry) = dir_entries.next_entry().await? {
            let file_path = entry.path();
            if file_path.extension().and_then(|s| s.to_str()) == Some("jsonl") {
                if let Ok(metadata) = entry.metadata().await {
                    if let Ok(modified) = metadata.modified() {
                        let modified_datetime: DateTime<Utc> = modified.into();
                        if modified_datetime < cutoff_date {
                            if tokio::fs::remove_file(&file_path).await.is_ok() {
                                deleted_count += 1;
                                info!("Deleted old audit log file: {:?}", file_path);
                            }
                        }
                    }
                }
            }
        }

        Ok(deleted_count)
    }
}

impl FileAuditStorage {
    /// Check if an audit entry matches the query
    fn matches_query(&self, entry: &AuditLogEntry, query: &AuditQuery) -> bool {
        // Date range filter
        if let Some(start) = query.start_date {
            if entry.timestamp < start {
                return false;
            }
        }
        if let Some(end) = query.end_date {
            if entry.timestamp > end {
                return false;
            }
        }

        // Event type filter
        if !query.event_types.is_empty() && !query.event_types.contains(&entry.event.event_type) {
            return false;
        }

        // Severity filter
        if !query.severities.is_empty() && !query.severities.contains(&entry.event.severity) {
            return false;
        }

        // User ID filter
        if !query.user_ids.is_empty() {
            if let Some(user_id) = &entry.event.context.user_id {
                if !query.user_ids.contains(user_id) {
                    return false;
                }
            } else {
                return false;
            }
        }

        // Repository ID filter
        if !query.repository_ids.is_empty() {
            if let Some(repo_id) = entry.event.repository_id {
                if !query.repository_ids.contains(&repo_id) {
                    return false;
                }
            } else {
                return false;
            }
        }

        true
    }
}

/// Audit logger for security events
pub struct AuditLogger {
    /// Storage backend
    storage: Arc<dyn AuditStorage>,
    /// Async channel for log entries
    sender: mpsc::UnboundedSender<AuditLogEntry>,
    /// Configuration
    config: Arc<RwLock<AuditConfig>>,
}

impl AuditLogger {
    /// Create a new audit logger
    pub fn new(storage: Arc<dyn AuditStorage>, config: AuditConfig) -> Self {
        let (sender, mut receiver) = mpsc::unbounded_channel::<AuditLogEntry>();
        let storage_clone = storage.clone();

        // Spawn background task to process audit log entries
        tokio::spawn(async move {
            while let Some(entry) = receiver.recv().await {
                if let Err(e) = storage_clone.store(&entry).await {
                    error!("Failed to store audit log entry: {}", e);
                }
            }
        });

        Self {
            storage,
            sender,
            config: Arc::new(RwLock::new(config)),
        }
    }

    /// Log a security event
    pub async fn log_event(&self, event: SecurityEvent) -> Result<()> {
        let config = self.config.read().await;
        
        if !config.enabled {
            return Ok(());
        }

        // Check log level filtering
        let should_log = match (&config.log_level, &event.severity) {
            (AuditLogLevel::Critical, SecurityEventSeverity::Critical) => true,
            (AuditLogLevel::Error, SecurityEventSeverity::Critical | SecurityEventSeverity::Error) => true,
            (AuditLogLevel::Warn, SecurityEventSeverity::Critical | SecurityEventSeverity::Error | SecurityEventSeverity::Warning) => true,
            (AuditLogLevel::Info, _) => true,
            (AuditLogLevel::Debug, _) => true,
            _ => false,
        };

        if !should_log {
            return Ok();
        }

        let entry = AuditLogEntry {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            event,
            metadata: AuditMetadata::default(),
        };

        self.sender.send(entry)
            .map_err(|_| anyhow::anyhow!("Failed to send audit log entry to processor"))?;

        Ok(())
    }

    /// Query audit logs
    pub async fn query(&self, query: AuditQuery) -> Result<Vec<AuditLogEntry>> {
        self.storage.query(&query).await
    }

    /// Export audit logs
    pub async fn export(&self, format: AuditExportFormat, query: AuditQuery) -> Result<Vec<u8>> {
        self.storage.export(format, &query).await
    }

    /// Clean up old audit logs
    pub async fn cleanup(&self) -> Result<u64> {
        let config = self.config.read().await;
        self.storage.cleanup(config.retention_days).await
    }

    /// Update audit configuration
    pub async fn update_config(&self, config: AuditConfig) -> Result<()> {
        let mut current_config = self.config.write().await;
        *current_config = config;
        Ok(())
    }

    /// Get audit statistics
    pub async fn get_statistics(&self, days: u32) -> Result<AuditStatistics> {
        let end_date = Utc::now();
        let start_date = end_date - chrono::Duration::days(days as i64);

        let query = AuditQuery {
            start_date: Some(start_date),
            end_date: Some(end_date),
            ..Default::default()
        };

        let entries = self.storage.query(&query).await?;
        
        let mut stats = AuditStatistics {
            total_events: entries.len(),
            events_by_type: HashMap::new(),
            events_by_severity: HashMap::new(),
            events_by_day: HashMap::new(),
            unique_users: std::collections::HashSet::new(),
            unique_repositories: std::collections::HashSet::new(),
        };

        for entry in entries {
            // Count by type
            *stats.events_by_type.entry(entry.event.event_type.clone()).or_insert(0) += 1;
            
            // Count by severity
            *stats.events_by_severity.entry(entry.event.severity.clone()).or_insert(0) += 1;
            
            // Count by day
            let day = entry.timestamp.format("%Y-%m-%d").to_string();
            *stats.events_by_day.entry(day).or_insert(0) += 1;
            
            // Track unique users and repositories
            if let Some(user_id) = entry.event.context.user_id {
                stats.unique_users.insert(user_id);
            }
            if let Some(repo_id) = entry.event.repository_id {
                stats.unique_repositories.insert(repo_id);
            }
        }

        Ok(stats)
    }
}

/// Audit statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditStatistics {
    /// Total number of events
    pub total_events: usize,
    /// Events by type
    pub events_by_type: HashMap<SecurityEventType, u64>,
    /// Events by severity
    pub events_by_severity: HashMap<SecurityEventSeverity, u64>,
    /// Events by day
    pub events_by_day: HashMap<String, u64>,
    /// Unique users
    #[serde(skip)]
    pub unique_users: std::collections::HashSet<String>,
    /// Unique repositories
    #[serde(skip)]
    pub unique_repositories: std::collections::HashSet<i32>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_file_audit_storage() {
        let temp_dir = TempDir::new().unwrap();
        let storage = FileAuditStorage::new(temp_dir.path().to_path_buf()).await.unwrap();

        let context = SecurityContext::system();
        let event = SecurityEvent::new(
            SecurityEventType::Authentication,
            SecurityEventSeverity::Info,
            "Test audit event".to_string(),
            context,
        );

        let entry = AuditLogEntry {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            event,
            metadata: AuditMetadata::default(),
        };

        // Store entry
        storage.store(&entry).await.unwrap();

        // Query entries
        let query = AuditQuery::default();
        let entries = storage.query(&query).await.unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].id, entry.id);
    }

    #[tokio::test]
    async fn test_audit_logger() {
        let temp_dir = TempDir::new().unwrap();
        let storage = Arc::new(FileAuditStorage::new(temp_dir.path().to_path_buf()).await.unwrap());
        let config = AuditConfig::default();
        let logger = AuditLogger::new(storage, config);

        let context = SecurityContext::system();
        let event = SecurityEvent::new(
            SecurityEventType::Authentication,
            SecurityEventSeverity::Info,
            "Test authentication event".to_string(),
            context,
        );

        // Log event
        logger.log_event(event).await.unwrap();

        // Wait a moment for async processing
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Query events
        let query = AuditQuery::default();
        let entries = logger.query(query).await.unwrap();
        assert_eq!(entries.len(), 1);
    }
}