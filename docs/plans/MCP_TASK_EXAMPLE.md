# MCP-Powered Task Example

This document provides a concrete example of how a Ratchet task would use MCP capabilities to interact with LLMs.

## Example: AI-Powered Code Review Task

This task demonstrates using MCP to perform automated code reviews using an LLM with code analysis tools.

### Task Structure

```
code-review-task/
├── metadata.json
├── main.js
├── input.schema.json
├── output.schema.json
└── tests/
    └── test-001.json
```

### metadata.json

```json
{
  "id": "ai-code-review",
  "version": "1.0.0",
  "label": "AI Code Review",
  "description": "Performs automated code review using LLM analysis",
  "author": "Ratchet Team",
  "tags": ["ai", "code-review", "development"],
  "requirements": {
    "mcp": ["claude", "code-analyzer"]
  }
}
```

### input.schema.json

```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "type": "object",
  "properties": {
    "repository": {
      "type": "string",
      "description": "Git repository URL"
    },
    "branch": {
      "type": "string",
      "description": "Branch to review",
      "default": "main"
    },
    "files": {
      "type": "array",
      "items": {
        "type": "object",
        "properties": {
          "path": {"type": "string"},
          "content": {"type": "string"}
        },
        "required": ["path", "content"]
      },
      "description": "Files to review"
    },
    "reviewType": {
      "type": "string",
      "enum": ["security", "performance", "style", "comprehensive"],
      "default": "comprehensive"
    },
    "severity": {
      "type": "string", 
      "enum": ["all", "high", "medium", "low"],
      "default": "all"
    }
  },
  "required": ["files"]
}
```

### output.schema.json

```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "type": "object",
  "properties": {
    "summary": {
      "type": "string",
      "description": "Overall review summary"
    },
    "score": {
      "type": "number",
      "minimum": 0,
      "maximum": 100,
      "description": "Code quality score"
    },
    "issues": {
      "type": "array",
      "items": {
        "type": "object",
        "properties": {
          "file": {"type": "string"},
          "line": {"type": "number"},
          "severity": {
            "type": "string",
            "enum": ["high", "medium", "low", "info"]
          },
          "type": {
            "type": "string",
            "enum": ["bug", "security", "performance", "style", "best-practice"]
          },
          "message": {"type": "string"},
          "suggestion": {"type": "string"}
        },
        "required": ["file", "severity", "type", "message"]
      }
    },
    "improvements": {
      "type": "array",
      "items": {
        "type": "string"
      },
      "description": "Suggested improvements"
    },
    "metrics": {
      "type": "object",
      "properties": {
        "complexity": {"type": "number"},
        "maintainability": {"type": "number"},
        "testCoverage": {"type": "number"}
      }
    }
  },
  "required": ["summary", "score", "issues"]
}
```

### main.js

```javascript
(async function(input) {
    // Initialize results
    const results = {
        issues: [],
        improvements: [],
        metrics: {}
    };
    
    try {
        // Step 1: Use code analyzer tool to get static analysis
        console.log("Running static code analysis...");
        const staticAnalysis = await mcp.invokeTool('code-analyzer', 'analyze', {
            files: input.files,
            checks: ['complexity', 'security', 'best-practices']
        });
        
        // Step 2: Prepare context for LLM review
        const codeContext = input.files.map(file => 
            `File: ${file.path}\n\`\`\`\n${file.content}\n\`\`\``
        ).join('\n\n');
        
        // Step 3: Get LLM review based on review type
        console.log(`Performing ${input.reviewType} review...`);
        const reviewPrompt = buildReviewPrompt(input.reviewType, codeContext, staticAnalysis);
        
        const llmReview = await mcp.complete('claude', {
            messages: [
                {
                    role: 'system',
                    content: 'You are an expert code reviewer. Provide detailed, actionable feedback.'
                },
                {
                    role: 'user',
                    content: reviewPrompt
                }
            ],
            max_tokens: 4000,
            temperature: 0.3
        });
        
        // Step 4: Parse LLM response
        const parsedReview = JSON.parse(llmReview.content);
        
        // Step 5: Combine static analysis with LLM insights
        results.issues = combineIssues(
            staticAnalysis.issues,
            parsedReview.issues,
            input.severity
        );
        
        // Step 6: Generate improvement suggestions
        if (results.issues.length > 0) {
            const improvementPrompt = `Based on these issues, suggest 3-5 high-level improvements:\n${
                JSON.stringify(results.issues, null, 2)
            }`;
            
            const improvements = await mcp.complete('claude', {
                messages: [{
                    role: 'user',
                    content: improvementPrompt
                }],
                max_tokens: 1000
            });
            
            results.improvements = JSON.parse(improvements.content).suggestions;
        }
        
        // Step 7: Calculate metrics and score
        results.metrics = {
            complexity: staticAnalysis.metrics.complexity,
            maintainability: calculateMaintainability(results.issues),
            testCoverage: staticAnalysis.metrics.testCoverage || 0
        };
        
        results.score = calculateQualityScore(results);
        results.summary = generateSummary(results, input.files.length);
        
        return results;
        
    } catch (error) {
        console.error("Code review failed:", error);
        throw new Error(`Failed to complete code review: ${error.message}`);
    }
    
    // Helper functions
    function buildReviewPrompt(reviewType, codeContext, staticAnalysis) {
        const prompts = {
            security: `Review this code for security vulnerabilities. Focus on:
                - Input validation
                - Authentication/authorization issues  
                - Injection vulnerabilities
                - Sensitive data exposure
                
                Static analysis found: ${JSON.stringify(staticAnalysis.securityIssues)}
                
                Code to review:
                ${codeContext}
                
                Return a JSON object with an "issues" array.`,
                
            performance: `Review this code for performance issues. Focus on:
                - Algorithmic complexity
                - Database query optimization
                - Memory leaks
                - Caching opportunities
                
                Code to review:
                ${codeContext}
                
                Return a JSON object with an "issues" array.`,
                
            comprehensive: `Perform a comprehensive code review covering:
                - Bugs and logic errors
                - Security vulnerabilities
                - Performance issues
                - Code style and best practices
                - Maintainability concerns
                
                Static analysis results: ${JSON.stringify(staticAnalysis, null, 2)}
                
                Code to review:
                ${codeContext}
                
                Return a JSON object with an "issues" array containing objects with:
                - file: filename
                - line: line number (if applicable)
                - severity: "high", "medium", or "low"
                - type: "bug", "security", "performance", "style", or "best-practice"
                - message: description of the issue
                - suggestion: how to fix it`
        };
        
        return prompts[reviewType] || prompts.comprehensive;
    }
    
    function combineIssues(staticIssues, llmIssues, severityFilter) {
        const combined = [...staticIssues, ...llmIssues];
        
        // Remove duplicates based on file + line + message similarity
        const unique = combined.filter((issue, index, self) =>
            index === self.findIndex(i => 
                i.file === issue.file && 
                Math.abs(i.line - issue.line) < 3 &&
                similarity(i.message, issue.message) > 0.8
            )
        );
        
        // Filter by severity if requested
        if (severityFilter !== 'all') {
            return unique.filter(issue => issue.severity === severityFilter);
        }
        
        return unique;
    }
    
    function calculateQualityScore(results) {
        let score = 100;
        
        // Deduct points based on issue severity
        results.issues.forEach(issue => {
            switch(issue.severity) {
                case 'high': score -= 10; break;
                case 'medium': score -= 5; break;
                case 'low': score -= 2; break;
                case 'info': score -= 0.5; break;
            }
        });
        
        // Factor in metrics
        score *= (results.metrics.maintainability / 100);
        score *= (1 - results.metrics.complexity / 100);
        
        return Math.max(0, Math.round(score));
    }
    
    function generateSummary(results, fileCount) {
        const issueCounts = results.issues.reduce((acc, issue) => {
            acc[issue.severity] = (acc[issue.severity] || 0) + 1;
            return acc;
        }, {});
        
        if (results.score >= 90) {
            return `Excellent code quality! Reviewed ${fileCount} files and found ${
                results.issues.length} minor issues. The code is well-structured and follows best practices.`;
        } else if (results.score >= 70) {
            return `Good code quality with room for improvement. Found ${issueCounts.high || 0} high, ${
                issueCounts.medium || 0} medium, and ${issueCounts.low || 0} low severity issues across ${
                fileCount} files.`;
        } else {
            return `Code needs significant improvements. Found ${results.issues.length} issues including ${
                issueCounts.high || 0} high severity problems. See detailed feedback below.`;
        }
    }
    
    function calculateMaintainability(issues) {
        // Simple maintainability score based on issue types
        const maintainabilityIssues = issues.filter(i => 
            i.type === 'style' || i.type === 'best-practice'
        ).length;
        
        return Math.max(0, 100 - (maintainabilityIssues * 5));
    }
    
    function similarity(str1, str2) {
        // Simple string similarity for deduplication
        const longer = str1.length > str2.length ? str1 : str2;
        const shorter = str1.length > str2.length ? str2 : str1;
        
        if (longer.length === 0) return 1.0;
        
        const editDistance = getEditDistance(longer, shorter);
        return (longer.length - editDistance) / longer.length;
    }
    
    function getEditDistance(s1, s2) {
        // Simplified edit distance
        s1 = s1.toLowerCase();
        s2 = s2.toLowerCase();
        
        const costs = [];
        for (let i = 0; i <= s1.length; i++) {
            let lastValue = i;
            for (let j = 0; j <= s2.length; j++) {
                if (i === 0) {
                    costs[j] = j;
                } else if (j > 0) {
                    let newValue = costs[j - 1];
                    if (s1.charAt(i - 1) !== s2.charAt(j - 1)) {
                        newValue = Math.min(Math.min(newValue, lastValue), costs[j]) + 1;
                    }
                    costs[j - 1] = lastValue;
                    lastValue = newValue;
                }
            }
            if (i > 0) costs[s2.length] = lastValue;
        }
        return costs[s2.length];
    }
})
```

### tests/test-001.json

```json
{
  "input": {
    "files": [
      {
        "path": "auth.js",
        "content": "function authenticate(username, password) {\n  const user = db.query(`SELECT * FROM users WHERE username = '${username}'`);\n  if (user && user.password === password) {\n    return { success: true, token: generateToken(user) };\n  }\n  return { success: false };\n}"
      }
    ],
    "reviewType": "security",
    "severity": "high"
  },
  "expected_output": {
    "score": 40,
    "issues": [
      {
        "file": "auth.js",
        "line": 2,
        "severity": "high",
        "type": "security",
        "message": "SQL injection vulnerability in user query",
        "suggestion": "Use parameterized queries or prepared statements"
      },
      {
        "file": "auth.js",
        "line": 3,
        "severity": "high",
        "type": "security",
        "message": "Passwords should not be stored in plain text",
        "suggestion": "Use bcrypt or similar for password hashing"
      }
    ]
  },
  "mock": {
    "mcp": {
      "code-analyzer": {
        "analyze": {
          "issues": [
            {
              "file": "auth.js",
              "line": 2,
              "severity": "high",
              "type": "security",
              "message": "SQL injection vulnerability detected"
            }
          ],
          "metrics": {
            "complexity": 15,
            "testCoverage": 0
          }
        }
      },
      "claude": {
        "complete": [
          {
            "content": "{\"issues\": [{\"file\": \"auth.js\", \"line\": 2, \"severity\": \"high\", \"type\": \"security\", \"message\": \"SQL injection vulnerability in user query\", \"suggestion\": \"Use parameterized queries or prepared statements\"}, {\"file\": \"auth.js\", \"line\": 3, \"severity\": \"high\", \"type\": \"security\", \"message\": \"Passwords should not be stored in plain text\", \"suggestion\": \"Use bcrypt or similar for password hashing\"}]}"
          }
        ]
      }
    }
  }
}
```

## Benefits of MCP Integration

This example demonstrates several key benefits:

1. **Tool Composition**: Combines static analysis tools with LLM insights
2. **Structured Output**: LLM responses are parsed into structured data
3. **Context Management**: Efficiently manages code context for LLM analysis  
4. **Error Handling**: Graceful fallbacks if MCP servers are unavailable
5. **Testability**: Can mock MCP responses for consistent testing

## Additional Use Cases

### 1. Data Analysis Task
```javascript
// Analyze CSV data using LLM
const insights = await mcp.complete('claude', {
    messages: [{
        role: 'user',
        content: `Analyze this dataset and provide insights: ${csvData}`
    }]
});
```

### 2. Content Generation Task
```javascript
// Generate marketing copy
const copy = await mcp.invokeTool('marketing-ai', 'generate-copy', {
    product: input.productDescription,
    tone: 'professional',
    length: 'medium'
});
```

### 3. Translation Task
```javascript
// Translate with context awareness
const translated = await mcp.complete('gpt4', {
    messages: [{
        role: 'system',
        content: 'You are a technical translator specializing in software documentation.'
    }, {
        role: 'user', 
        content: `Translate to ${input.targetLanguage}: ${input.text}`
    }]
});
```

### 4. Code Generation Task
```javascript
// Generate code from specifications
const code = await mcp.invokeTool('codegen-server', 'generate', {
    specification: input.spec,
    language: 'typescript',
    style: 'functional'
});
```

## Conclusion

MCP integration enables Ratchet tasks to leverage powerful AI capabilities while maintaining the platform's security and reliability guarantees. The combination of structured task definitions, schema validation, and MCP's standardized protocol creates a robust foundation for AI-powered automation.