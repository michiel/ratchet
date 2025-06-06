//! Safe query filtering to prevent SQL injection

use crate::{StorageError, StorageResult};
use regex::Regex;

/// Safe filter builder for preventing SQL injection
pub struct SafeFilterBuilder {
    filters: Vec<Filter>,
    allowed_fields: Vec<String>,
    max_conditions: usize,
}

/// A single filter condition
#[derive(Debug, Clone)]
pub struct Filter {
    pub field: String,
    pub operator: FilterOperator,
    pub value: FilterValue,
}

/// Filter operators
#[derive(Debug, Clone)]
pub enum FilterOperator {
    Equals,
    NotEquals,
    Like,
    In,
    NotIn,
    GreaterThan,
    LessThan,
    GreaterThanOrEqual,
    LessThanOrEqual,
    IsNull,
    IsNotNull,
    Between,
}

/// Filter values
#[derive(Debug, Clone)]
pub enum FilterValue {
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    Array(Vec<FilterValue>),
    Null,
    Range(Box<FilterValue>, Box<FilterValue>),
}

impl SafeFilterBuilder {
    /// Create a new safe filter builder
    pub fn new() -> Self {
        Self {
            filters: Vec::new(),
            allowed_fields: Vec::new(),
            max_conditions: 50, // Prevent DoS with too many conditions
        }
    }

    /// Set allowed fields that can be filtered
    pub fn with_allowed_fields(mut self, fields: Vec<String>) -> Self {
        self.allowed_fields = fields;
        self
    }

    /// Set maximum number of filter conditions
    pub fn with_max_conditions(mut self, max: usize) -> Self {
        self.max_conditions = max;
        self
    }

    /// Add an equality filter
    pub fn equals(mut self, field: impl Into<String>, value: FilterValue) -> StorageResult<Self> {
        self.add_filter(field.into(), FilterOperator::Equals, value)?;
        Ok(self)
    }

    /// Add a LIKE filter
    pub fn like(
        mut self,
        field: impl Into<String>,
        pattern: impl Into<String>,
    ) -> StorageResult<Self> {
        let pattern = self.sanitize_like_pattern(pattern.into())?;
        self.add_filter(
            field.into(),
            FilterOperator::Like,
            FilterValue::String(pattern),
        )?;
        Ok(self)
    }

    /// Add an IN filter
    pub fn in_values(
        mut self,
        field: impl Into<String>,
        values: Vec<FilterValue>,
    ) -> StorageResult<Self> {
        if values.is_empty() {
            return Err(StorageError::ValidationFailed(
                "IN filter cannot have empty values".to_string(),
            ));
        }
        if values.len() > 100 {
            return Err(StorageError::ValidationFailed(
                "IN filter cannot have more than 100 values".to_string(),
            ));
        }
        self.add_filter(field.into(), FilterOperator::In, FilterValue::Array(values))?;
        Ok(self)
    }

    /// Add a range filter
    pub fn between(
        mut self,
        field: impl Into<String>,
        start: FilterValue,
        end: FilterValue,
    ) -> StorageResult<Self> {
        self.add_filter(
            field.into(),
            FilterOperator::Between,
            FilterValue::Range(Box::new(start), Box::new(end)),
        )?;
        Ok(self)
    }

    /// Add a null check filter
    pub fn is_null(mut self, field: impl Into<String>) -> StorageResult<Self> {
        self.add_filter(field.into(), FilterOperator::IsNull, FilterValue::Null)?;
        Ok(self)
    }

    /// Add a not null check filter
    pub fn is_not_null(mut self, field: impl Into<String>) -> StorageResult<Self> {
        self.add_filter(field.into(), FilterOperator::IsNotNull, FilterValue::Null)?;
        Ok(self)
    }

    /// Build the filters
    pub fn build(self) -> StorageResult<Vec<Filter>> {
        Ok(self.filters)
    }

    /// Add a filter with validation
    fn add_filter(
        &mut self,
        field: String,
        operator: FilterOperator,
        value: FilterValue,
    ) -> StorageResult<()> {
        // Check max conditions limit
        if self.filters.len() >= self.max_conditions {
            return Err(StorageError::ValidationFailed(format!(
                "Too many filter conditions (max: {})",
                self.max_conditions
            )));
        }

        // Validate field name
        self.validate_field_name(&field)?;

        // Check if field is allowed
        if !self.allowed_fields.is_empty() && !self.allowed_fields.contains(&field) {
            return Err(StorageError::ValidationFailed(format!(
                "Field '{}' is not allowed for filtering",
                field
            )));
        }

        // Validate filter value
        self.validate_filter_value(&value)?;

        self.filters.push(Filter {
            field,
            operator,
            value,
        });

        Ok(())
    }

    /// Validate field name to prevent SQL injection
    fn validate_field_name(&self, field: &str) -> StorageResult<()> {
        // Allow only alphanumeric characters, underscores, and dots
        let field_regex = Regex::new(r"^[a-zA-Z_][a-zA-Z0-9_\.]*$").unwrap();

        if !field_regex.is_match(field) {
            return Err(StorageError::ValidationFailed(format!(
                "Invalid field name: {}",
                field
            )));
        }

        // Prevent common SQL injection patterns
        let lower_field = field.to_lowercase();
        let dangerous_patterns = [
            "select", "insert", "update", "delete", "drop", "union", "exec", "execute", "sp_",
            "xp_", "alter", "create",
        ];

        for pattern in &dangerous_patterns {
            if lower_field.contains(pattern) {
                return Err(StorageError::ValidationFailed(format!(
                    "Field name contains potentially dangerous pattern: {}",
                    pattern
                )));
            }
        }

        Ok(())
    }

    /// Validate filter value
    fn validate_filter_value(&self, value: &FilterValue) -> StorageResult<()> {
        match value {
            FilterValue::String(s) => {
                if s.len() > 1000 {
                    return Err(StorageError::ValidationFailed(
                        "String filter value too long (max: 1000 characters)".to_string(),
                    ));
                }

                // Check for potential SQL injection patterns
                let dangerous_patterns = ["'", "\"", ";", "--", "/*", "*/", "xp_", "sp_"];
                for pattern in &dangerous_patterns {
                    if s.contains(pattern) {
                        return Err(StorageError::ValidationFailed(format!(
                            "String contains potentially dangerous pattern: {}",
                            pattern
                        )));
                    }
                }
            }
            FilterValue::Array(values) => {
                for v in values {
                    self.validate_filter_value(v)?;
                }
            }
            FilterValue::Range(start, end) => {
                self.validate_filter_value(start)?;
                self.validate_filter_value(end)?;
            }
            _ => {} // Other types are safe
        }

        Ok(())
    }

    /// Sanitize LIKE pattern to prevent injection
    fn sanitize_like_pattern(&self, pattern: String) -> StorageResult<String> {
        // Escape special LIKE characters
        let escaped = pattern
            .replace("\\", "\\\\") // Escape backslashes first
            .replace("%", "\\%") // Escape percent
            .replace("_", "\\_"); // Escape underscore

        // Validate the escaped pattern
        if escaped.len() > 100 {
            return Err(StorageError::ValidationFailed(
                "LIKE pattern too long (max: 100 characters)".to_string(),
            ));
        }

        Ok(escaped)
    }
}

impl Default for SafeFilterBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Convert filter value to SQL parameter
impl FilterValue {
    pub fn to_sql_param(&self) -> String {
        match self {
            FilterValue::String(s) => format!("'{}'", s.replace("'", "''")),
            FilterValue::Integer(i) => i.to_string(),
            FilterValue::Float(f) => f.to_string(),
            FilterValue::Boolean(b) => if *b { "TRUE" } else { "FALSE" }.to_string(),
            FilterValue::Null => "NULL".to_string(),
            FilterValue::Array(_) => "?".to_string(), // Handled specially
            FilterValue::Range(_, _) => "?".to_string(), // Handled specially
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_safe_filter_builder() {
        let builder = SafeFilterBuilder::new()
            .with_allowed_fields(vec!["name".to_string(), "status".to_string()]);

        let result = builder
            .equals("name", FilterValue::String("test".to_string()))
            .unwrap()
            .like("status", "active")
            .unwrap()
            .build();

        assert!(result.is_ok());
        let filters = result.unwrap();
        assert_eq!(filters.len(), 2);
    }

    #[test]
    fn test_invalid_field_name() {
        let builder = SafeFilterBuilder::new();

        let result = builder.equals(
            "'; DROP TABLE users; --",
            FilterValue::String("test".to_string()),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_dangerous_string_value() {
        let builder = SafeFilterBuilder::new();

        let result = builder.equals(
            "name",
            FilterValue::String("'; DROP TABLE users; --".to_string()),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_max_conditions_limit() {
        let mut builder = SafeFilterBuilder::new().with_max_conditions(2);

        builder = builder
            .equals("field1", FilterValue::String("value1".to_string()))
            .unwrap();
        builder = builder
            .equals("field2", FilterValue::String("value2".to_string()))
            .unwrap();

        let result = builder.equals("field3", FilterValue::String("value3".to_string()));
        assert!(result.is_err());
    }

    #[test]
    fn test_allowed_fields() {
        let builder = SafeFilterBuilder::new().with_allowed_fields(vec!["name".to_string()]);

        // Allowed field should work
        let result = builder.equals("name", FilterValue::String("test".to_string()));
        assert!(result.is_ok());

        // Disallowed field should fail
        let builder = SafeFilterBuilder::new().with_allowed_fields(vec!["name".to_string()]);
        let result = builder.equals("email", FilterValue::String("test".to_string()));
        assert!(result.is_err());
    }
}
