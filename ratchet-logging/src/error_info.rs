use crate::severity::ErrorSeverity;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorInfo {
    pub error_type: String,
    pub error_code: String,
    pub message: String,
    pub severity: ErrorSeverity,
    pub is_retryable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stack_trace: Option<String>,
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub context: HashMap<String, JsonValue>,
    pub suggestions: ErrorSuggestions,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub related_errors: Vec<RelatedError>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ErrorSuggestions {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub immediate: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub preventive: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelatedError {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub error_type: String,
    pub message: String,
}

impl ErrorInfo {
    pub fn new(
        error_type: impl Into<String>,
        error_code: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            error_type: error_type.into(),
            error_code: error_code.into(),
            message: message.into(),
            severity: ErrorSeverity::Medium,
            is_retryable: false,
            stack_trace: None,
            context: HashMap::new(),
            suggestions: ErrorSuggestions::default(),
            related_errors: Vec::new(),
        }
    }

    pub fn with_severity(mut self, severity: ErrorSeverity) -> Self {
        self.severity = severity;
        self
    }

    pub fn with_retryable(mut self, is_retryable: bool) -> Self {
        self.is_retryable = is_retryable;
        self
    }

    pub fn with_stack_trace(mut self, stack_trace: impl Into<String>) -> Self {
        self.stack_trace = Some(stack_trace.into());
        self
    }

    pub fn with_context_value(mut self, key: impl Into<String>, value: impl Serialize) -> Self {
        if let Ok(json_value) = serde_json::to_value(value) {
            self.context.insert(key.into(), json_value);
        }
        self
    }

    pub fn with_suggestion(mut self, immediate: impl Into<String>) -> Self {
        self.suggestions.immediate.push(immediate.into());
        self
    }

    pub fn with_preventive_suggestion(mut self, preventive: impl Into<String>) -> Self {
        self.suggestions.preventive.push(preventive.into());
        self
    }

    pub fn with_related_error(mut self, error: RelatedError) -> Self {
        self.related_errors.push(error);
        self
    }
}
