//! Template engine for dynamic paths and URLs

use crate::errors::DeliveryError;
use handlebars::Handlebars;
use serde_json::Value;
use std::collections::HashMap;

/// Template engine for variable substitution using Handlebars
#[derive(Debug, Clone)]
pub struct TemplateEngine {
    handlebars: Handlebars<'static>,
}

impl TemplateEngine {
    pub fn new() -> Self {
        let mut handlebars = Handlebars::new();
        handlebars.set_strict_mode(true); // Error on missing variables
        
        Self {
            handlebars,
        }
    }

    /// Render a template with the given variables
    pub fn render(
        &self,
        template: &str,
        variables: &HashMap<String, String>,
    ) -> Result<String, DeliveryError> {
        // Convert HashMap<String, String> to Value for handlebars
        let json_vars: Value = variables.iter()
            .map(|(k, v)| (k.clone(), Value::String(v.clone())))
            .collect::<serde_json::Map<_, _>>()
            .into();

        self.handlebars
            .render_template(template, &json_vars)
            .map_err(|e| DeliveryError::TemplateRender {
                template: template.to_string(),
                error: e.to_string(),
            })
    }

    /// Render a template with JSON variables (for complex data)
    pub fn render_json(
        &self,
        template: &str,
        variables: &Value,
    ) -> Result<String, DeliveryError> {
        self.handlebars
            .render_template(template, variables)
            .map_err(|e| DeliveryError::TemplateRender {
                template: template.to_string(),
                error: e.to_string(),
            })
    }

    /// Validate that a template is syntactically correct
    pub fn validate(&self, template: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Try to compile the template to check syntax
        match handlebars::Template::compile(template) {
            Ok(_) => Ok(()),
            Err(e) => Err(format!("Invalid template syntax: {}", e).into()),
        }
    }

    /// Check if a template contains any variables
    pub fn has_variables(&self, template: &str) -> bool {
        // Simple check for handlebars syntax
        template.contains("{{") && template.contains("}}")
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
    use serde_json::json;

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
    fn test_json_rendering() {
        let engine = TemplateEngine::new();
        let vars = json!({
            "job": {
                "id": 123,
                "name": "test-job"
            },
            "env": "production"
        });

        let template = "/results/{{env}}/{{job.id}}/{{job.name}}.json";
        let result = engine.render_json(template, &vars).unwrap();

        assert_eq!(result, "/results/production/123/test-job.json");
    }

    #[test]
    fn test_missing_variable() {
        let engine = TemplateEngine::new();
        let vars = HashMap::new();

        let template = "/results/{{missing_var}}/output.json";
        let result = engine.render(template, &vars);

        assert!(result.is_err());
        match result.unwrap_err() {
            DeliveryError::TemplateRender { template: t, error: _ } => {
                assert_eq!(t, template);
            }
            _ => panic!("Expected TemplateRender error"),
        }
    }

    #[test]
    fn test_template_validation() {
        let engine = TemplateEngine::new();

        // Valid templates
        assert!(engine.validate("{{var1}}/{{var2}}").is_ok());
        assert!(engine.validate("no variables").is_ok());
        assert!(engine.validate("{{valid_name_123}}").is_ok());
        assert!(engine.validate("{{#if condition}}content{{/if}}").is_ok());

        // Invalid templates
        assert!(engine.validate("{{unmatched").is_err());
        assert!(engine.validate("{{}}").is_err());
    }

    #[test]
    fn test_has_variables() {
        let engine = TemplateEngine::new();

        assert!(engine.has_variables("{{var}}"));
        assert!(engine.has_variables("prefix/{{var}}/suffix"));
        assert!(engine.has_variables("{{#if condition}}content{{/if}}"));
        assert!(!engine.has_variables("no variables here"));
    }
}