# Input Validation Enhancement Proposal

## Problem Statement

The current input validation system incorrectly rejects legitimate cron expressions due to overly broad injection attack patterns. The pattern `\*\/` used to detect SQL comment injection also matches legitimate cron syntax like `*/2 * * * *`.

## Current Architecture

```rust
pub struct InputValidator {
    // Generic validation for all input types
    fn check_injection_patterns(&self, input: &str) -> Result<(), ValidationError>
}
```

## Proposed Architecture

### 1. Context-Aware Validation System

```rust
#[derive(Debug, Clone)]
pub enum InputType {
    CronExpression,
    Email,
    Url,
    TaskName,
    Description,
    Json,
    SqlQuery,      // For admin/debug interfaces
    ShellCommand,  // For system tasks
    FilePath,
    GenericString,
}

#[derive(Debug, Clone)]
pub struct ValidationContext {
    pub input_type: InputType,
    pub allow_patterns: Vec<String>,
    pub deny_patterns: Vec<String>,
    pub max_length: Option<usize>,
    pub required: bool,
}

pub trait InputTypeValidator {
    fn validate(&self, input: &str, context: &ValidationContext) -> Result<(), ValidationError>;
    fn sanitize(&self, input: &str) -> String;
}
```

### 2. Specific Validators

#### A. CronExpressionValidator
```rust
pub struct CronExpressionValidator;

impl InputTypeValidator for CronExpressionValidator {
    fn validate(&self, input: &str, _context: &ValidationContext) -> Result<(), ValidationError> {
        // 1. Parse with cron library to ensure valid syntax
        // 2. Check for reasonable bounds (not more frequent than every second)
        // 3. Allow legitimate cron patterns including */N syntax
        
        use cron::Schedule;
        let schedule = Schedule::from_str(input)
            .map_err(|_| ValidationError::InvalidFormat("Invalid cron expression".to_string()))?;
        
        // Additional safety checks
        if input.contains("* * * * * *") {
            return Err(ValidationError::InvalidFormat("Every second cron not allowed".to_string()));
        }
        
        Ok(())
    }
}
```

#### B. EmailValidator
```rust
pub struct EmailValidator;

impl InputTypeValidator for EmailValidator {
    fn validate(&self, input: &str, context: &ValidationContext) -> Result<(), ValidationError> {
        // Use email-specific validation
        // Allow @ and . which might be flagged by generic injection detection
        email_address::EmailAddress::from_str(input)
            .map_err(|_| ValidationError::InvalidFormat("Invalid email".to_string()))?;
        Ok(())
    }
}
```

#### C. SqlQueryValidator  
```rust
pub struct SqlQueryValidator;

impl InputTypeValidator for SqlQueryValidator {
    fn validate(&self, input: &str, context: &ValidationContext) -> Result<(), ValidationError> {
        // Strict SQL injection detection - only for admin interfaces
        // Apply the current injection patterns here
        self.check_sql_injection_patterns(input)?;
        Ok(())
    }
}
```

### 3. Enhanced InputValidator

```rust
#[derive(Debug, Clone)]
pub struct InputValidator {
    validators: HashMap<InputType, Box<dyn InputTypeValidator>>,
    // ... existing fields
}

impl InputValidator {
    pub fn validate_with_type(&self, input: &str, input_type: InputType) -> Result<(), ValidationError> {
        let context = ValidationContext {
            input_type: input_type.clone(),
            ..Default::default()
        };
        
        // Get specific validator for this input type
        if let Some(validator) = self.validators.get(&input_type) {
            validator.validate(input, &context)?;
        } else {
            // Fallback to generic validation (current behavior)
            self.validate_string(input)?;
        }
        
        Ok(())
    }
    
    // Legacy method for backward compatibility
    pub fn validate_string(&self, input: &str) -> Result<(), ValidationError> {
        self.validate_with_type(input, InputType::GenericString)
    }
}
```

### 4. Usage in Schedule Handler

```rust
// In ratchet-rest-api/src/handlers/schedules.rs
pub async fn create_schedule(
    ctx: AppContext,
    Json(request): Json<CreateScheduleRequest>,
) -> Result<Json<ScheduleResponse>, RestError> {
    
    // Context-aware validation
    ctx.validator.validate_with_type(&request.cron_expression, InputType::CronExpression)
        .map_err(|e| RestError::BadRequest(format!("Invalid cron expression: {}", e)))?;
        
    ctx.validator.validate_with_type(&request.name, InputType::TaskName)
        .map_err(|e| RestError::BadRequest(format!("Invalid name: {}", e)))?;
    
    // ... rest of handler
}
```

## Benefits

1. **Accuracy**: No false positives for legitimate input patterns
2. **Security**: Maintains protection against actual injection attacks  
3. **Extensibility**: Easy to add new input types and validators
4. **Maintainability**: Clear separation of validation logic by input type
5. **Backward Compatibility**: Existing code continues to work

## Implementation Strategy

### Phase 1: Core Framework
- Implement `InputType` enum and `ValidationContext`
- Create `InputTypeValidator` trait
- Extend `InputValidator` with type-aware validation

### Phase 2: Specific Validators
- Implement `CronExpressionValidator`
- Implement `EmailValidator` 
- Implement `UrlValidator`
- Move existing injection detection to `SqlQueryValidator`

### Phase 3: Integration
- Update REST API handlers to use typed validation
- Update GraphQL resolvers to use typed validation
- Add configuration for validation rules per input type

### Phase 4: Advanced Features
- Configurable validation rules via config files
- Context-sensitive validation (admin vs user inputs)
- Validation rule hot-reloading

## Security Considerations

- Default to strict validation for unknown input types
- Maintain audit logging for validation failures
- Allow allowlisting of specific patterns for each input type
- Regular review of validation rules and patterns