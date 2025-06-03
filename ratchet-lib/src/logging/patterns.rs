use super::ErrorInfo;
use serde::{Deserialize, Serialize};
use regex::Regex;

/// Error pattern for matching and categorizing errors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorPattern {
    /// Unique identifier for the pattern
    pub id: String,
    
    /// Human-readable name
    pub name: String,
    
    /// Description of what this pattern represents
    pub description: String,
    
    /// Category of errors this pattern matches
    pub category: ErrorCategory,
    
    /// Rules for matching this pattern
    pub matching_rules: Vec<MatchingRule>,
    
    /// Suggested immediate actions
    pub suggestions: Vec<String>,
    
    /// Suggested preventive measures
    pub preventive_measures: Vec<String>,
    
    /// Related documentation or resources
    pub related_documentation: Vec<String>,
    
    /// Common root causes
    pub common_causes: Vec<String>,
    
    /// LLM analysis prompts specific to this pattern
    pub llm_prompts: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCategory {
    Network,
    Database,
    Authentication,
    Configuration,
    Resource,
    Validation,
    TaskExecution,
    System,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MatchingRule {
    /// Match error type exactly
    ErrorType { value: String },
    
    /// Match error code exactly
    ErrorCode { value: String },
    
    /// Match message with regex
    MessagePattern { pattern: String },
    
    /// Match if field exists and equals value
    FieldEquals { field: String, value: serde_json::Value },
    
    /// Match if field exists and matches regex
    FieldPattern { field: String, pattern: String },
    
    /// Match if all sub-rules match
    All { rules: Vec<MatchingRule> },
    
    /// Match if any sub-rule matches
    Any { rules: Vec<MatchingRule> },
    
    /// Match if the sub-rule does not match
    Not { rule: Box<MatchingRule> },
}

impl ErrorPattern {
    /// Check if this pattern matches the given error
    pub fn matches(&self, error: &ErrorInfo) -> bool {
        // All matching rules must match (implicit AND)
        self.matching_rules.iter().all(|rule| rule.matches(error))
    }
    
    /// Calculate a match score (0.0 to 1.0) for ranking patterns
    pub fn match_score(&self, error: &ErrorInfo) -> f64 {
        if !self.matches(error) {
            return 0.0;
        }
        
        // Calculate score based on specificity of rules
        let base_score = 0.5;
        let specificity_bonus = self.matching_rules.len() as f64 * 0.1;
        
        (base_score + specificity_bonus).min(1.0)
    }
}

impl MatchingRule {
    /// Check if this rule matches the given error
    pub fn matches(&self, error: &ErrorInfo) -> bool {
        match self {
            Self::ErrorType { value } => &error.error_type == value,
            
            Self::ErrorCode { value } => &error.error_code == value,
            
            Self::MessagePattern { pattern } => {
                match Regex::new(pattern) {
                    Ok(re) => re.is_match(&error.message),
                    Err(_) => false,
                }
            }
            
            Self::FieldEquals { field, value } => {
                error.context.get(field).map_or(false, |v| v == value)
            }
            
            Self::FieldPattern { field, pattern } => {
                error.context.get(field).and_then(|v| v.as_str()).is_some_and(|s| {
                    match Regex::new(pattern) {
                        Ok(re) => re.is_match(s),
                        Err(_) => false,
                    }
                })
            }
            
            Self::All { rules } => rules.iter().all(|r| r.matches(error)),
            
            Self::Any { rules } => rules.iter().any(|r| r.matches(error)),
            
            Self::Not { rule } => !rule.matches(error),
        }
    }
}

/// Error pattern matcher for finding matching patterns
pub struct ErrorPatternMatcher {
    patterns: Vec<ErrorPattern>,
}

impl ErrorPatternMatcher {
    pub fn new(patterns: Vec<ErrorPattern>) -> Self {
        Self { patterns }
    }
    
    /// Load patterns from built-in definitions
    pub fn with_defaults() -> Self {
        Self::new(Self::default_patterns())
    }
    
    /// Find the best matching pattern for an error
    pub fn match_error(&self, error: &ErrorInfo) -> Option<&ErrorPattern> {
        self.patterns.iter()
            .filter(|p| p.matches(error))
            .max_by(|a, b| {
                a.match_score(error)
                    .partial_cmp(&b.match_score(error))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    }
    
    /// Find all matching patterns for an error
    pub fn match_all(&self, error: &ErrorInfo) -> Vec<&ErrorPattern> {
        self.patterns.iter()
            .filter(|p| p.matches(error))
            .collect()
    }
    
    /// Get suggestions from matching patterns
    pub fn get_suggestions(&self, error: &ErrorInfo) -> Vec<String> {
        let mut suggestions = Vec::new();
        
        for pattern in self.match_all(error) {
            suggestions.extend(pattern.suggestions.clone());
        }
        
        // Remove duplicates while preserving order
        let mut seen = std::collections::HashSet::new();
        suggestions.retain(|s| seen.insert(s.clone()));
        
        suggestions
    }
    
    /// Default error patterns for common scenarios
    fn default_patterns() -> Vec<ErrorPattern> {
        vec![
            // Database connection timeout
            ErrorPattern {
                id: "db_connection_timeout".to_string(),
                name: "Database Connection Timeout".to_string(),
                description: "Failed to establish database connection within timeout period".to_string(),
                category: ErrorCategory::Database,
                matching_rules: vec![
                    MatchingRule::ErrorCode { value: "DB_CONN_ERROR".to_string() },
                    MatchingRule::MessagePattern { 
                        pattern: r"(?i)(timeout|timed out)".to_string() 
                    },
                ],
                suggestions: vec![
                    "Check database server is running and accessible".to_string(),
                    "Verify network connectivity to database host".to_string(),
                    "Check firewall rules allow database port".to_string(),
                ],
                preventive_measures: vec![
                    "Implement connection pooling with health checks".to_string(),
                    "Add circuit breaker for database connections".to_string(),
                    "Configure appropriate connection timeout values".to_string(),
                ],
                related_documentation: vec![
                    "https://docs.ratchet.io/database-configuration".to_string(),
                ],
                common_causes: vec![
                    "Database server down or overloaded".to_string(),
                    "Network issues between application and database".to_string(),
                    "Incorrect connection string or credentials".to_string(),
                ],
                llm_prompts: vec![
                    "Analyze database connection timeout in distributed system".to_string(),
                    "What are common causes of database connection timeouts?".to_string(),
                ],
            },
            
            // Task not found
            ErrorPattern {
                id: "task_not_found".to_string(),
                name: "Task Not Found".to_string(),
                description: "Requested task does not exist in the registry".to_string(),
                category: ErrorCategory::TaskExecution,
                matching_rules: vec![
                    MatchingRule::ErrorType { value: "TaskNotFound".to_string() },
                ],
                suggestions: vec![
                    "Verify task name is correct".to_string(),
                    "Run 'ratchet list' to see available tasks".to_string(),
                    "Check if task file exists and is properly formatted".to_string(),
                ],
                preventive_measures: vec![
                    "Implement task name validation at input".to_string(),
                    "Add task discovery endpoint to API".to_string(),
                    "Create task name autocomplete functionality".to_string(),
                ],
                related_documentation: vec![
                    "https://docs.ratchet.io/task-management".to_string(),
                ],
                common_causes: vec![
                    "Typo in task name".to_string(),
                    "Task file not deployed or loaded".to_string(),
                    "Task disabled or removed".to_string(),
                ],
                llm_prompts: vec![
                    "How to handle missing tasks in a task execution system?".to_string(),
                ],
            },
            
            // HTTP timeout
            ErrorPattern {
                id: "http_timeout".to_string(),
                name: "HTTP Request Timeout".to_string(),
                description: "HTTP request failed due to timeout".to_string(),
                category: ErrorCategory::Network,
                matching_rules: vec![
                    MatchingRule::Any { rules: vec![
                        MatchingRule::ErrorCode { value: "NETWORK_TIMEOUT".to_string() },
                        MatchingRule::All { rules: vec![
                            MatchingRule::ErrorType { value: "NetworkError".to_string() },
                            MatchingRule::MessagePattern { 
                                pattern: r"(?i)timeout".to_string() 
                            },
                        ]},
                    ]},
                ],
                suggestions: vec![
                    "Check if the remote service is responsive".to_string(),
                    "Increase HTTP timeout configuration".to_string(),
                    "Verify network path to remote service".to_string(),
                ],
                preventive_measures: vec![
                    "Implement retry logic with exponential backoff".to_string(),
                    "Add circuit breaker for external services".to_string(),
                    "Cache responses when appropriate".to_string(),
                ],
                related_documentation: vec![
                    "https://docs.ratchet.io/http-configuration".to_string(),
                ],
                common_causes: vec![
                    "Remote service overloaded or down".to_string(),
                    "Network latency or packet loss".to_string(),
                    "Timeout value too low for operation".to_string(),
                ],
                llm_prompts: vec![
                    "Best practices for handling HTTP timeouts in microservices".to_string(),
                ],
            },
            
            // Rate limiting
            ErrorPattern {
                id: "rate_limited".to_string(),
                name: "Rate Limit Exceeded".to_string(),
                description: "Request rejected due to rate limiting".to_string(),
                category: ErrorCategory::Network,
                matching_rules: vec![
                    MatchingRule::Any { rules: vec![
                        MatchingRule::ErrorCode { value: "RATE_LIMITED".to_string() },
                        MatchingRule::MessagePattern { 
                            pattern: r"(?i)(rate limit|too many requests|429)".to_string() 
                        },
                    ]},
                ],
                suggestions: vec![
                    "Reduce request frequency".to_string(),
                    "Check rate limit headers for reset time".to_string(),
                    "Implement request queuing".to_string(),
                ],
                preventive_measures: vec![
                    "Implement client-side rate limiting".to_string(),
                    "Use exponential backoff for retries".to_string(),
                    "Cache API responses when possible".to_string(),
                ],
                related_documentation: vec![
                    "https://docs.ratchet.io/rate-limiting".to_string(),
                ],
                common_causes: vec![
                    "Exceeding API rate limits".to_string(),
                    "Burst of requests from application".to_string(),
                    "Shared rate limit across multiple clients".to_string(),
                ],
                llm_prompts: vec![
                    "How to handle rate limiting gracefully in distributed systems?".to_string(),
                ],
            },
        ]
    }
}

/// Pattern analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternAnalysis {
    /// The pattern that was matched
    pub pattern: ErrorPattern,
    
    /// Match confidence score (0.0 to 1.0)
    pub confidence: f64,
    
    /// Similar errors in recent history
    pub similar_errors_count: usize,
    
    /// Trend analysis
    pub trend: ErrorTrend,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorTrend {
    Increasing,
    Stable,
    Decreasing,
    Spike,
    New,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::errors::ErrorSeverity;
    
    #[test]
    fn test_pattern_matching() {
        let pattern = ErrorPattern {
            id: "test_pattern".to_string(),
            name: "Test Pattern".to_string(),
            description: "Test".to_string(),
            category: ErrorCategory::Database,
            matching_rules: vec![
                MatchingRule::ErrorType { value: "DatabaseError".to_string() },
                MatchingRule::MessagePattern { pattern: r"timeout".to_string() },
            ],
            suggestions: vec!["Test suggestion".to_string()],
            preventive_measures: vec![],
            related_documentation: vec![],
            common_causes: vec![],
            llm_prompts: vec![],
        };
        
        let error = ErrorInfo::new("DatabaseError", "DB_TIMEOUT", "Connection timeout after 5s")
            .with_severity(ErrorSeverity::High);
        
        assert!(pattern.matches(&error));
        assert!(pattern.match_score(&error) > 0.5);
    }
    
    #[test]
    fn test_complex_matching_rules() {
        let rule = MatchingRule::All { rules: vec![
            MatchingRule::ErrorType { value: "NetworkError".to_string() },
            MatchingRule::Any { rules: vec![
                MatchingRule::MessagePattern { pattern: r"timeout".to_string() },
                MatchingRule::MessagePattern { pattern: r"refused".to_string() },
            ]},
        ]};
        
        let error1 = ErrorInfo::new("NetworkError", "NET_001", "Connection timeout");
        assert!(rule.matches(&error1));
        
        let error2 = ErrorInfo::new("NetworkError", "NET_002", "Connection refused");
        assert!(rule.matches(&error2));
        
        let error3 = ErrorInfo::new("NetworkError", "NET_003", "Unknown host");
        assert!(!rule.matches(&error3));
        
        let error4 = ErrorInfo::new("DatabaseError", "DB_001", "Connection timeout");
        assert!(!rule.matches(&error4));
    }
    
    #[test]
    fn test_pattern_matcher() {
        let matcher = ErrorPatternMatcher::with_defaults();
        
        let error = ErrorInfo::new("TaskNotFound", "TASK_NOT_FOUND", "Task 'foo' not found");
        let pattern = matcher.match_error(&error);
        
        assert!(pattern.is_some());
        assert_eq!(pattern.unwrap().id, "task_not_found");
        
        let suggestions = matcher.get_suggestions(&error);
        assert!(!suggestions.is_empty());
        assert!(suggestions.iter().any(|s| s.contains("ratchet list")));
    }
}