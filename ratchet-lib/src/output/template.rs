//! Template engine for dynamic paths and URLs

use crate::output::errors::DeliveryError;
use regex::Regex;
use std::collections::HashMap;

/// Simple template engine for variable substitution
#[derive(Debug, Clone)]
pub struct TemplateEngine {
    variable_regex: Regex,
}

impl TemplateEngine {
    pub fn new() -> Self {
        Self {
            // Matches {{variable_name}} patterns
            variable_regex: Regex::new(r"\{\{([a-zA-Z_][a-zA-Z0-9_]*)\}\}")
                .expect("Valid regex pattern"),
        }
    }

    /// Render a template with the given variables
    pub fn render(
        &self,
        template: &str,
        variables: &HashMap<String, String>,
    ) -> Result<String, DeliveryError> {
        let mut result = template.to_string();

        // Find all variable references
        for capture in self.variable_regex.captures_iter(template) {
            let full_match = &capture[0]; // {{variable_name}}
            let variable_name = &capture[1]; // variable_name

            if let Some(value) = variables.get(variable_name) {
                result = result.replace(full_match, value);
            } else {
                return Err(DeliveryError::InvalidTemplateVariable {
                    variable: variable_name.to_string(),
                });
            }
        }

        Ok(result)
    }

    /// Validate that a template contains only valid variable references
    pub fn validate(&self, template: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Check for basic template syntax errors
        let mut brace_count = 0;
        let chars: Vec<char> = template.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            if i + 1 < chars.len() && chars[i] == '{' && chars[i + 1] == '{' {
                brace_count += 1;
                i += 2; // Skip both braces
            } else if i + 1 < chars.len() && chars[i] == '}' && chars[i + 1] == '}' {
                if brace_count > 0 {
                    brace_count -= 1;
                    i += 2; // Skip both braces
                } else {
                    return Err("Unmatched closing braces }}".into());
                }
            } else if chars[i] == '{' || chars[i] == '}' {
                // Single brace not part of {{ or }}
                return Err(
                    format!("Single brace '{}' not allowed, use {{{{ or }}}}", chars[i]).into(),
                );
            } else {
                i += 1;
            }
        }

        if brace_count != 0 {
            return Err("Unmatched template braces".into());
        }

        // Validate variable names
        for capture in self.variable_regex.captures_iter(template) {
            let variable_name = &capture[1];
            if variable_name.is_empty() {
                return Err("Empty variable name".into());
            }

            // Check for valid variable name (letters, numbers, underscores)
            if !variable_name
                .chars()
                .all(|c| c.is_alphanumeric() || c == '_')
            {
                return Err(format!("Invalid variable name: {}", variable_name).into());
            }
        }

        // Check for any {{ }} patterns that don't contain valid variable names
        // This catches things like {{}} or {{123}} that don't match our variable pattern
        let empty_var_regex = Regex::new(r"\{\{[^}]*\}\}").unwrap();
        for m in empty_var_regex.find_iter(template) {
            let matched = m.as_str();
            // If this {{ }} pattern didn't match our variable regex, it's invalid
            if !self.variable_regex.is_match(matched) {
                return Err(format!("Invalid template variable: {}", matched).into());
            }
        }

        Ok(())
    }

    /// Extract all variable names from a template
    pub fn extract_variables(&self, template: &str) -> Vec<String> {
        self.variable_regex
            .captures_iter(template)
            .map(|capture| capture[1].to_string())
            .collect()
    }

    /// Check if a template contains any variables
    pub fn has_variables(&self, template: &str) -> bool {
        self.variable_regex.is_match(template)
    }
}

impl Default for TemplateEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_template_rendering() {
        let engine = TemplateEngine::new();
        let mut vars = HashMap::new();
        vars.insert("job_id".to_string(), "123".to_string());
        vars.insert("timestamp".to_string(), "20240106_143000".to_string());
        vars.insert("env".to_string(), "production".to_string());

        let template = "/results/{{env}}/{{job_id}}/{{timestamp}}.json";
        let result = engine.render(template, &vars).unwrap();

        assert_eq!(result, "/results/production/123/20240106_143000.json");
    }

    #[test]
    fn test_missing_variable() {
        let engine = TemplateEngine::new();
        let vars = HashMap::new();

        let template = "/results/{{missing_var}}/output.json";
        let result = engine.render(template, &vars);

        assert!(result.is_err());
        match result.unwrap_err() {
            DeliveryError::InvalidTemplateVariable { variable } => {
                assert_eq!(variable, "missing_var");
            }
            _ => panic!("Expected InvalidTemplateVariable error"),
        }
    }

    #[test]
    fn test_template_validation() {
        let engine = TemplateEngine::new();

        // Valid templates
        assert!(engine.validate("{{var1}}/{{var2}}").is_ok());
        assert!(engine.validate("no variables").is_ok());
        assert!(engine.validate("{{valid_name_123}}").is_ok());

        // Invalid templates
        assert!(engine.validate("{{unmatched").is_err());
        assert!(engine.validate("unmatched}}").is_err());
        assert!(engine.validate("{{}}").is_err());
        assert!(engine.validate("{{invalid-name}}").is_err());
    }

    #[test]
    fn test_extract_variables() {
        let engine = TemplateEngine::new();
        let template = "{{var1}}/{{var2}}/{{var1}}"; // var1 appears twice
        let variables = engine.extract_variables(template);

        assert_eq!(variables, vec!["var1", "var2", "var1"]);
    }

    #[test]
    fn test_has_variables() {
        let engine = TemplateEngine::new();

        assert!(engine.has_variables("{{var}}"));
        assert!(engine.has_variables("prefix/{{var}}/suffix"));
        assert!(!engine.has_variables("no variables here"));
    }
}
