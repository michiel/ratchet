use sea_orm::{entity::*, ColumnTrait, Condition, EntityTrait, Value};
use sea_orm::sea_query::SimpleExpr;

/// Safe filter builder for database queries
/// Prevents SQL injection by properly escaping and parameterizing queries
pub struct SafeFilterBuilder<E: EntityTrait> {
    conditions: Vec<SimpleExpr>,
    _phantom: std::marker::PhantomData<E>,
}

impl<E: EntityTrait> SafeFilterBuilder<E> {
    pub fn new() -> Self {
        Self {
            conditions: Vec::new(),
            _phantom: std::marker::PhantomData,
        }
    }

    /// Add a LIKE filter with proper escaping
    pub fn add_like_filter<C>(&mut self, column: C, value: &str) -> &mut Self
    where
        C: ColumnTrait,
    {
        if !value.is_empty() {
            // Escape special characters in LIKE patterns
            let escaped = value
                .replace('\\', "\\\\")
                .replace('%', "\\%")
                .replace('_', "\\_");
            
            // Use contains for safety (adds % on both sides)
            self.conditions.push(column.contains(&escaped));
        }
        self
    }

    /// Add an exact match filter
    pub fn add_exact_filter<C, V>(&mut self, column: C, value: V) -> &mut Self
    where
        C: ColumnTrait,
        V: Into<Value>,
    {
        self.conditions.push(column.eq(value));
        self
    }
    
    /// Add a condition directly (for complex expressions like is_null/is_not_null)
    pub fn add_condition(&mut self, condition: SimpleExpr) -> &mut Self {
        self.conditions.push(condition);
        self
    }

    /// Add an optional exact match filter
    pub fn add_optional_filter<C, V>(&mut self, column: C, value: Option<V>) -> &mut Self
    where
        C: ColumnTrait,
        V: Into<Value>,
    {
        if let Some(v) = value {
            self.conditions.push(column.eq(v));
        }
        self
    }

    /// Add a range filter
    pub fn add_range_filter<C, V>(&mut self, column: C, min: Option<V>, max: Option<V>) -> &mut Self
    where
        C: ColumnTrait,
        V: Into<Value> + Clone,
    {
        if let Some(min_val) = min {
            self.conditions.push(column.gte(min_val));
        }
        if let Some(max_val) = max {
            self.conditions.push(column.lte(max_val));
        }
        self
    }

    /// Add an IN filter for multiple values
    pub fn add_in_filter<C, V, I>(&mut self, column: C, values: I) -> &mut Self
    where
        C: ColumnTrait,
        V: Into<Value>,
        I: IntoIterator<Item = V>,
    {
        let values_vec: Vec<Value> = values.into_iter().map(Into::into).collect();
        if !values_vec.is_empty() {
            self.conditions.push(column.is_in(values_vec));
        }
        self
    }

    /// Build the final condition
    pub fn build(self) -> Condition {
        self.conditions
            .into_iter()
            .fold(Condition::all(), |cond, expr| cond.add(expr))
    }
}

/// Input validation for preventing SQL injection
pub mod validation {
    use thiserror::Error;

    #[derive(Error, Debug)]
    pub enum ValidationError {
        #[error("SQL injection attempt detected")]
        SqlInjectionAttempt,
        #[error("Invalid input: {0}")]
        InvalidInput(String),
    }

    /// Validate input for potential SQL injection patterns
    pub fn validate_query_input(input: &str) -> Result<(), ValidationError> {
        // Common SQL injection patterns to block
        const FORBIDDEN_PATTERNS: &[&str] = &[
            "--;", "/*", "*/", "xp_", "sp_", "0x", "@@", "char(", "nchar(",
            "alter", "begin", "cast", "create", "cursor", "declare", "delete",
            "drop", "exec", "execute", "fetch", "insert", "kill", "select",
            "sys", "sysobjects", "syscolumns", "table", "update", "union",
            "script", "<script", "javascript:", "vbscript:", "onload=", "onerror=",
            "onclick=", "onmouseover=", "<iframe", "<frame", "<embed", "<object",
            " or ", " and ", "=", "'", "\"",
        ];

        let lower_input = input.to_lowercase();
        
        for pattern in FORBIDDEN_PATTERNS {
            if lower_input.contains(pattern) {
                return Err(ValidationError::SqlInjectionAttempt);
            }
        }

        // Check for common encoding attempts
        if input.contains("&#") || input.contains("\\x") || input.contains("\\u") {
            return Err(ValidationError::SqlInjectionAttempt);
        }

        Ok(())
    }

    /// Sanitize input by removing potentially dangerous characters
    pub fn sanitize_input(input: &str) -> String {
        input
            .chars()
            .filter(|c| c.is_alphanumeric() || matches!(c, ' ' | '-' | '_' | '.' | '@'))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sql_injection_detection() {
        assert!(validation::validate_query_input("normal input").is_ok());
        assert!(validation::validate_query_input("test@example.com").is_ok());
        
        assert!(validation::validate_query_input("'; DROP TABLE users--").is_err());
        assert!(validation::validate_query_input("1 OR 1=1").is_err());
        assert!(validation::validate_query_input("<script>alert('xss')</script>").is_err());
    }

    #[test]
    fn test_input_sanitization() {
        assert_eq!(validation::sanitize_input("normal-input_123"), "normal-input_123");
        assert_eq!(validation::sanitize_input("test@example.com"), "test@example.com");
        assert_eq!(validation::sanitize_input("'; DROP TABLE--"), " DROP TABLE--");
    }
}