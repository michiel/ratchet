---
layout: default
title: Example Uses
permalink: /examples/
---

# Example Uses

This page demonstrates various use cases and examples of how to use Ratchet for different task orchestration scenarios.

## Table of Contents

- [Basic Task Examples](#basic-task-examples)
- [API Integration Examples](#api-integration-examples)
- [Data Processing Examples](#data-processing-examples)
- [Scheduled Job Examples](#scheduled-job-examples)
- [Error Handling Examples](#error-handling-examples)

## Basic Task Examples

### Simple Addition Task

A basic task that performs addition:

```javascript
// sample/js-tasks/addition/main.js
(function(input) {
    const a = input.a || 0;
    const b = input.b || 0;
    
    return {
        sum: a + b,
        operation: `${a} + ${b} = ${a + b}`
    };
})
```

**Input Schema** (`input.schema.json`):
```json
{
    "$schema": "http://json-schema.org/draft-07/schema#",
    "type": "object",
    "properties": {
        "a": { "type": "number" },
        "b": { "type": "number" }
    },
    "required": ["a", "b"]
}
```

**Execution via CLI**:
```bash
# Direct execution
ratchet-cli execute addition --input '{"a": 5, "b": 3}'

# Using test file
ratchet-cli test addition --test-file tests/test-001.json
```

**Execution via REST API**:
```bash
curl -X POST http://localhost:8080/api/v1/tasks/addition/execute \
  -H "Content-Type: application/json" \
  -d '{"input": {"a": 5, "b": 3}}'
```

## API Integration Examples

### Weather API Integration

Fetch weather data from an external API:

```javascript
// sample/js-tasks/weather-api/main.js
(function(input) {
    const city = input.city || "Unknown";
    const units = input.units || "metric";
    
    const API_KEY = "your-api-key-here";
    const url = `https://api.openweathermap.org/data/2.5/weather?q=${encodeURIComponent(city)}&units=${units}&appid=${API_KEY}`;
    
    try {
        const response = fetch(url, { method: "GET" });
        const data = response.body;
        
        return {
            location: `${data.name}, ${data.sys.country}`,
            temperature: data.main.temp,
            units: units === "metric" ? "C" : "F",
            description: data.weather[0].description,
            humidity: data.main.humidity
        };
    } catch (error) {
        throw new NetworkError(`Failed to fetch weather data: ${error.message}`);
    }
})
```

### REST API Call with Authentication

Make authenticated API calls:

```javascript
// sample/js-tasks/rest-call-sample/main.js
(function(input) {
    const endpoint = input.endpoint;
    const method = input.method || "GET";
    const headers = input.headers || {};
    const body = input.body;
    
    // Add authentication if provided
    if (input.auth_token) {
        headers["Authorization"] = `Bearer ${input.auth_token}`;
    }
    
    const options = {
        method: method,
        headers: headers
    };
    
    if (body && method !== "GET") {
        options.body = JSON.stringify(body);
        headers["Content-Type"] = "application/json";
    }
    
    try {
        const response = fetch(endpoint, options);
        
        return {
            status: response.status,
            headers: response.headers,
            body: response.body,
            success: response.status >= 200 && response.status < 300
        };
    } catch (error) {
        throw new NetworkError(`API call failed: ${error.message}`);
    }
})
```

## Data Processing Examples

### CSV Data Processing

Process CSV data and transform it:

```javascript
(function(input) {
    const csvData = input.csv_data;
    const delimiter = input.delimiter || ",";
    
    // Parse CSV
    const lines = csvData.split("\n").filter(line => line.trim());
    const headers = lines[0].split(delimiter);
    const rows = lines.slice(1).map(line => {
        const values = line.split(delimiter);
        return headers.reduce((obj, header, index) => {
            obj[header.trim()] = values[index]?.trim();
            return obj;
        }, {});
    });
    
    // Example transformation: calculate statistics
    const stats = {
        total_rows: rows.length,
        columns: headers,
        sample_data: rows.slice(0, 3)
    };
    
    return {
        processed_rows: rows.length,
        statistics: stats,
        data: rows
    };
})
```

### JSON Data Transformation

Transform and validate JSON data:

```javascript
(function(input) {
    const data = input.data;
    const transformations = input.transformations || [];
    
    let result = data;
    
    for (const transform of transformations) {
        switch (transform.type) {
            case "filter":
                result = result.filter(item => 
                    item[transform.field] === transform.value
                );
                break;
                
            case "map":
                result = result.map(item => ({
                    ...item,
                    [transform.target]: item[transform.source]
                }));
                break;
                
            case "aggregate":
                const sum = result.reduce((acc, item) => 
                    acc + (item[transform.field] || 0), 0
                );
                result = { sum, count: result.length, average: sum / result.length };
                break;
        }
    }
    
    return {
        original_count: data.length,
        transformed_data: result,
        transformations_applied: transformations.length
    };
})
```

## Scheduled Job Examples

### Daily Report Generation

Create a task for scheduled report generation:

```javascript
(function(input) {
    const reportType = input.report_type || "daily";
    const date = input.date || new Date().toISOString().split('T')[0];
    
    // Simulate fetching data from database
    const metrics = {
        total_users: Math.floor(Math.random() * 1000) + 500,
        active_sessions: Math.floor(Math.random() * 200) + 50,
        transactions: Math.floor(Math.random() * 500) + 100,
        revenue: (Math.random() * 10000 + 5000).toFixed(2)
    };
    
    // Generate report
    const report = {
        type: reportType,
        date: date,
        generated_at: new Date().toISOString(),
        metrics: metrics,
        summary: `${reportType} report for ${date}: ${metrics.transactions} transactions totaling $${metrics.revenue}`
    };
    
    return report;
})
```

**Schedule via CLI**:
```bash
# Create a daily schedule
ratchet-cli schedule create \
  --task daily-report \
  --cron "0 9 * * *" \
  --input '{"report_type": "daily"}'
```

**Schedule via REST API**:
```bash
curl -X POST http://localhost:8080/api/v1/schedules \
  -H "Content-Type: application/json" \
  -d '{
    "task_id": "daily-report",
    "cron_expression": "0 9 * * *",
    "input": {"report_type": "daily"},
    "enabled": true
  }'
```

## Error Handling Examples

### Robust Error Handling

Task with comprehensive error handling:

```javascript
(function(input) {
    // Validate required inputs
    if (!input.url) {
        throw new ValidationError("URL is required");
    }
    
    if (!input.url.startsWith("http://") && !input.url.startsWith("https://")) {
        throw new ValidationError("URL must start with http:// or https://");
    }
    
    try {
        // Attempt to fetch data
        const response = fetch(input.url, {
            method: "GET",
            timeout: input.timeout || 30000
        });
        
        // Check response status
        if (response.status === 404) {
            throw new NotFoundError(`Resource not found at ${input.url}`);
        }
        
        if (response.status >= 500) {
            throw new NetworkError(`Server error: ${response.status}`);
        }
        
        if (response.status >= 400) {
            throw new ValidationError(`Client error: ${response.status}`);
        }
        
        // Process response
        const data = response.body;
        
        // Validate response data
        if (!data || typeof data !== 'object') {
            throw new DataError("Invalid response format");
        }
        
        return {
            success: true,
            data: data,
            metadata: {
                url: input.url,
                status: response.status,
                timestamp: new Date().toISOString()
            }
        };
        
    } catch (error) {
        // Re-throw typed errors
        if (error.name && error.name.endsWith('Error')) {
            throw error;
        }
        
        // Wrap unknown errors
        throw new RuntimeError(`Unexpected error: ${error.message}`);
    }
})
```

### Retry with Backoff

Task that implements retry logic:

```javascript
(function(input) {
    const maxRetries = input.max_retries || 3;
    const baseDelay = input.base_delay || 1000;
    
    let lastError = null;
    
    for (let attempt = 0; attempt <= maxRetries; attempt++) {
        try {
            // Simulate an operation that might fail
            if (Math.random() < 0.7 && attempt < maxRetries) {
                throw new Error("Temporary failure");
            }
            
            // Success!
            return {
                success: true,
                attempts: attempt + 1,
                message: "Operation completed successfully"
            };
            
        } catch (error) {
            lastError = error;
            
            if (attempt < maxRetries) {
                // Calculate exponential backoff
                const delay = baseDelay * Math.pow(2, attempt);
                
                // In a real implementation, you would use setTimeout
                // For this example, we just log the retry
                console.log(`Retry ${attempt + 1}/${maxRetries} after ${delay}ms`);
            }
        }
    }
    
    // All retries failed
    throw new RuntimeError(`Operation failed after ${maxRetries + 1} attempts: ${lastError.message}`);
})
```

## Best Practices

### 1. Input Validation
Always validate inputs at the beginning of your task:

```javascript
if (!input.required_field) {
    throw new ValidationError("required_field is missing");
}
```

### 2. Error Types
Use appropriate error types for different scenarios:
- `ValidationError`: Invalid input data
- `NetworkError`: Network-related failures
- `DataError`: Invalid data format or structure
- `NotFoundError`: Resource not found
- `RuntimeError`: General runtime errors

### 3. Resource Cleanup
Ensure resources are properly cleaned up:

```javascript
try {
    // Perform operations
} finally {
    // Cleanup code here
}
```

### 4. Logging
Use console.log for debugging information that will be captured in execution logs:

```javascript
console.log("Processing started", { input_size: input.data.length });
```

### 5. Testing
Always include test files with your tasks:

```json
// tests/test-001.json
{
    "name": "Basic test case",
    "input": {
        "field1": "value1",
        "field2": 123
    },
    "expected_output": {
        "result": "expected_value"
    }
}
```

## Next Steps

- Learn about [Server Configuration]({{ "/server-configuration" | relative_url }}) to deploy these examples
- Explore [Integrations]({{ "/integrations" | relative_url }}) for connecting with external systems
- Understand [Logging & Error Handling]({{ "/logging-error-handling" | relative_url }}) for debugging