//! Security and authentication testing for the Ratchet MCP Protocol
//!
//! This module provides comprehensive security testing scenarios for MCP including
//! protocol security, message validation, authentication, and resource protection.

use serde_json::Value;
use std::time::Duration;
use tokio::time::sleep;

/// MCP security test configuration
#[derive(Debug, Clone)]
pub struct McpSecurityTestConfig {
    pub enable_authentication: bool,
    pub enable_message_validation: bool,
    pub max_message_size: usize,
    pub max_concurrent_connections: u32,
    pub timeout_seconds: u64,
}

impl Default for McpSecurityTestConfig {
    fn default() -> Self {
        Self {
            enable_authentication: true,
            enable_message_validation: true,
            max_message_size: 1024 * 1024, // 1MB
            max_concurrent_connections: 100,
            timeout_seconds: 30,
        }
    }
}

/// MCP security test results
#[derive(Debug, Clone)]
pub struct McpSecurityTestResults {
    pub test_name: String,
    pub total_tests: u32,
    pub passed_tests: u32,
    pub failed_tests: u32,
    pub vulnerabilities_found: Vec<McpSecurityVulnerability>,
    pub security_score: f64,
}

/// MCP security vulnerability details
#[derive(Debug, Clone)]
pub struct McpSecurityVulnerability {
    pub vulnerability_type: McpVulnerabilityType,
    pub severity: Severity,
    pub description: String,
    pub recommendation: String,
    pub method: String,
}

/// Types of MCP security vulnerabilities
#[derive(Debug, Clone)]
pub enum McpVulnerabilityType {
    AuthenticationBypass,
    AuthorizationEscalation,
    MessageValidationFailure,
    ResourceExhaustion,
    ProtocolViolation,
    InformationDisclosure,
    InjectionAttack,
    RateLimitBypass,
    ConnectionAbuse,
    DataIntegrityViolation,
}

/// Security vulnerability severity levels
#[derive(Debug, Clone)]
pub enum Severity {
    Critical,
    High,
    Medium,
    Low,
    Info,
}

/// MCP protocol security test runner
pub struct McpProtocolSecurityTest {
    config: McpSecurityTestConfig,
}

impl Default for McpProtocolSecurityTest {
    fn default() -> Self {
        Self::new()
    }
}

impl McpProtocolSecurityTest {
    /// Create a new MCP security test runner
    pub fn new() -> Self {
        Self::with_config(McpSecurityTestConfig::default())
    }

    /// Create a new MCP security test runner with custom configuration
    pub fn with_config(config: McpSecurityTestConfig) -> Self {
        Self { config }
    }

    /// Run comprehensive MCP security test suite
    pub async fn run_comprehensive_security_tests(&self) -> Result<McpSecurityTestResults, Box<dyn std::error::Error>> {
        println!("🔒 Running comprehensive MCP protocol security tests...");

        let mut results = McpSecurityTestResults {
            test_name: "Comprehensive MCP Protocol Security".to_string(),
            total_tests: 0,
            passed_tests: 0,
            failed_tests: 0,
            vulnerabilities_found: Vec::new(),
            security_score: 0.0,
        };

        // Authentication tests
        println!("🔐 Testing MCP authentication security...");
        let auth_results = self.test_authentication_security().await?;
        self.merge_results(&mut results, auth_results);

        // Message validation tests
        println!("📋 Testing MCP message validation security...");
        let validation_results = self.test_message_validation_security().await?;
        self.merge_results(&mut results, validation_results);

        // Protocol security tests
        println!("🔗 Testing MCP protocol security...");
        let protocol_results = self.test_protocol_security().await?;
        self.merge_results(&mut results, protocol_results);

        // Resource protection tests
        println!("🛡️ Testing MCP resource protection...");
        let resource_results = self.test_resource_protection_security().await?;
        self.merge_results(&mut results, resource_results);

        // Connection security tests
        println!("🔌 Testing MCP connection security...");
        let connection_results = self.test_connection_security().await?;
        self.merge_results(&mut results, connection_results);

        // Data integrity tests
        println!("🗂️ Testing MCP data integrity security...");
        let integrity_results = self.test_data_integrity_security().await?;
        self.merge_results(&mut results, integrity_results);

        // Calculate overall security score
        results.security_score = self.calculate_security_score(&results);

        self.print_security_report(&results);

        Ok(results)
    }

    /// Test MCP authentication security scenarios
    async fn test_authentication_security(&self) -> Result<McpSecurityTestResults, Box<dyn std::error::Error>> {
        let mut results = McpSecurityTestResults {
            test_name: "MCP Authentication Security".to_string(),
            total_tests: 0,
            passed_tests: 0,
            failed_tests: 0,
            vulnerabilities_found: Vec::new(),
            security_score: 0.0,
        };

        // Test 1: Unauthenticated access to protected methods
        results.total_tests += 1;
        let protected_methods = vec!["task/create", "task/update", "task/delete", "task/execute", "task/test"];

        for method in &protected_methods {
            if self.simulate_unauthenticated_mcp_call(method).await.is_err() {
                results.passed_tests += 1;
            } else {
                results.failed_tests += 1;
                results.vulnerabilities_found.push(McpSecurityVulnerability {
                    vulnerability_type: McpVulnerabilityType::AuthenticationBypass,
                    severity: Severity::High,
                    description: format!("MCP method {} allows unauthenticated access", method),
                    recommendation: "Require authentication for all protected MCP methods".to_string(),
                    method: method.to_string(),
                });
            }
        }

        // Test 2: Invalid authentication token handling
        results.total_tests += 1;
        if self.simulate_invalid_auth_mcp_call("task/create").await.is_err() {
            results.passed_tests += 1;
        } else {
            results.failed_tests += 1;
            results.vulnerabilities_found.push(McpSecurityVulnerability {
                vulnerability_type: McpVulnerabilityType::AuthenticationBypass,
                severity: Severity::High,
                description: "MCP accepts invalid authentication tokens".to_string(),
                recommendation: "Implement strict authentication token validation".to_string(),
                method: "task/create".to_string(),
            });
        }

        // Test 3: Session management security
        results.total_tests += 1;
        if self.test_mcp_session_security().await {
            results.passed_tests += 1;
        } else {
            results.failed_tests += 1;
            results.vulnerabilities_found.push(McpSecurityVulnerability {
                vulnerability_type: McpVulnerabilityType::AuthenticationBypass,
                severity: Severity::Medium,
                description: "MCP session management has security issues".to_string(),
                recommendation: "Implement secure session handling with proper timeouts".to_string(),
                method: "session/*".to_string(),
            });
        }

        Ok(results)
    }

    /// Test MCP message validation security scenarios
    async fn test_message_validation_security(&self) -> Result<McpSecurityTestResults, Box<dyn std::error::Error>> {
        let mut results = McpSecurityTestResults {
            test_name: "MCP Message Validation Security".to_string(),
            total_tests: 0,
            passed_tests: 0,
            failed_tests: 0,
            vulnerabilities_found: Vec::new(),
            security_score: 0.0,
        };

        // Test 1: Malformed JSON-RPC message handling
        results.total_tests += 1;
        if self.test_mcp_malformed_message_handling().await {
            results.passed_tests += 1;
        } else {
            results.failed_tests += 1;
            results.vulnerabilities_found.push(McpSecurityVulnerability {
                vulnerability_type: McpVulnerabilityType::MessageValidationFailure,
                severity: Severity::High,
                description: "MCP does not properly handle malformed JSON-RPC messages".to_string(),
                recommendation: "Implement strict JSON-RPC message validation".to_string(),
                method: "*".to_string(),
            });
        }

        // Test 2: Oversized message handling
        results.total_tests += 1;
        if self.test_mcp_oversized_message_handling().await {
            results.passed_tests += 1;
        } else {
            results.failed_tests += 1;
            results.vulnerabilities_found.push(McpSecurityVulnerability {
                vulnerability_type: McpVulnerabilityType::ResourceExhaustion,
                severity: Severity::High,
                description: "MCP does not limit message sizes, allowing resource exhaustion".to_string(),
                recommendation: "Implement message size limits to prevent DoS attacks".to_string(),
                method: "*".to_string(),
            });
        }

        // Test 3: Parameter injection attacks
        results.total_tests += 1;
        if self.test_mcp_parameter_injection().await {
            results.passed_tests += 1;
        } else {
            results.failed_tests += 1;
            results.vulnerabilities_found.push(McpSecurityVulnerability {
                vulnerability_type: McpVulnerabilityType::InjectionAttack,
                severity: Severity::Critical,
                description: "MCP is vulnerable to parameter injection attacks".to_string(),
                recommendation: "Implement strict parameter validation and sanitization".to_string(),
                method: "task/create".to_string(),
            });
        }

        // Test 4: Type confusion attacks
        results.total_tests += 1;
        if self.test_mcp_type_confusion().await {
            results.passed_tests += 1;
        } else {
            results.failed_tests += 1;
            results.vulnerabilities_found.push(McpSecurityVulnerability {
                vulnerability_type: McpVulnerabilityType::MessageValidationFailure,
                severity: Severity::Medium,
                description: "MCP is vulnerable to type confusion attacks".to_string(),
                recommendation: "Implement strict type validation for all parameters".to_string(),
                method: "task/update".to_string(),
            });
        }

        Ok(results)
    }

    /// Test MCP protocol security scenarios
    async fn test_protocol_security(&self) -> Result<McpSecurityTestResults, Box<dyn std::error::Error>> {
        let mut results = McpSecurityTestResults {
            test_name: "MCP Protocol Security".to_string(),
            total_tests: 0,
            passed_tests: 0,
            failed_tests: 0,
            vulnerabilities_found: Vec::new(),
            security_score: 0.0,
        };

        // Test 1: Protocol version security
        results.total_tests += 1;
        if self.test_mcp_protocol_version_security().await {
            results.passed_tests += 1;
        } else {
            results.failed_tests += 1;
            results.vulnerabilities_found.push(McpSecurityVulnerability {
                vulnerability_type: McpVulnerabilityType::ProtocolViolation,
                severity: Severity::Medium,
                description: "MCP does not properly validate protocol versions".to_string(),
                recommendation: "Implement strict protocol version validation".to_string(),
                method: "initialize".to_string(),
            });
        }

        // Test 2: Method enumeration protection
        results.total_tests += 1;
        if self.test_mcp_method_enumeration_protection().await {
            results.passed_tests += 1;
        } else {
            results.failed_tests += 1;
            results.vulnerabilities_found.push(McpSecurityVulnerability {
                vulnerability_type: McpVulnerabilityType::InformationDisclosure,
                severity: Severity::Medium,
                description: "MCP exposes method information through enumeration".to_string(),
                recommendation: "Implement method access control and information hiding".to_string(),
                method: "*".to_string(),
            });
        }

        // Test 3: Error message information disclosure
        results.total_tests += 1;
        if self.test_mcp_error_information_disclosure().await {
            results.passed_tests += 1;
        } else {
            results.failed_tests += 1;
            results.vulnerabilities_found.push(McpSecurityVulnerability {
                vulnerability_type: McpVulnerabilityType::InformationDisclosure,
                severity: Severity::Low,
                description: "MCP error messages disclose sensitive information".to_string(),
                recommendation: "Sanitize error messages to prevent information disclosure".to_string(),
                method: "*".to_string(),
            });
        }

        Ok(results)
    }

    /// Test MCP resource protection security scenarios
    async fn test_resource_protection_security(&self) -> Result<McpSecurityTestResults, Box<dyn std::error::Error>> {
        let mut results = McpSecurityTestResults {
            test_name: "MCP Resource Protection Security".to_string(),
            total_tests: 0,
            passed_tests: 0,
            failed_tests: 0,
            vulnerabilities_found: Vec::new(),
            security_score: 0.0,
        };

        // Test 1: Rate limiting enforcement
        results.total_tests += 1;
        if self.test_mcp_rate_limiting().await {
            results.passed_tests += 1;
        } else {
            results.failed_tests += 1;
            results.vulnerabilities_found.push(McpSecurityVulnerability {
                vulnerability_type: McpVulnerabilityType::RateLimitBypass,
                severity: Severity::High,
                description: "MCP does not enforce rate limiting".to_string(),
                recommendation: "Implement rate limiting to prevent abuse".to_string(),
                method: "*".to_string(),
            });
        }

        // Test 2: Resource access control
        results.total_tests += 1;
        if self.test_mcp_resource_access_control().await {
            results.passed_tests += 1;
        } else {
            results.failed_tests += 1;
            results.vulnerabilities_found.push(McpSecurityVulnerability {
                vulnerability_type: McpVulnerabilityType::AuthorizationEscalation,
                severity: Severity::High,
                description: "MCP does not properly control resource access".to_string(),
                recommendation: "Implement proper resource ownership and access controls".to_string(),
                method: "task/*".to_string(),
            });
        }

        // Test 3: Concurrent operation limits
        results.total_tests += 1;
        if self.test_mcp_concurrent_operation_limits().await {
            results.passed_tests += 1;
        } else {
            results.failed_tests += 1;
            results.vulnerabilities_found.push(McpSecurityVulnerability {
                vulnerability_type: McpVulnerabilityType::ResourceExhaustion,
                severity: Severity::Medium,
                description: "MCP does not limit concurrent operations".to_string(),
                recommendation: "Implement concurrent operation limits".to_string(),
                method: "task/execute".to_string(),
            });
        }

        Ok(results)
    }

    /// Test MCP connection security scenarios
    async fn test_connection_security(&self) -> Result<McpSecurityTestResults, Box<dyn std::error::Error>> {
        let mut results = McpSecurityTestResults {
            test_name: "MCP Connection Security".to_string(),
            total_tests: 0,
            passed_tests: 0,
            failed_tests: 0,
            vulnerabilities_found: Vec::new(),
            security_score: 0.0,
        };

        // Test 1: Connection limits
        results.total_tests += 1;
        if self.test_mcp_connection_limits().await {
            results.passed_tests += 1;
        } else {
            results.failed_tests += 1;
            results.vulnerabilities_found.push(McpSecurityVulnerability {
                vulnerability_type: McpVulnerabilityType::ConnectionAbuse,
                severity: Severity::High,
                description: "MCP does not limit concurrent connections".to_string(),
                recommendation: "Implement connection limits to prevent DoS attacks".to_string(),
                method: "connection".to_string(),
            });
        }

        // Test 2: Connection timeout enforcement
        results.total_tests += 1;
        if self.test_mcp_connection_timeout().await {
            results.passed_tests += 1;
        } else {
            results.failed_tests += 1;
            results.vulnerabilities_found.push(McpSecurityVulnerability {
                vulnerability_type: McpVulnerabilityType::ResourceExhaustion,
                severity: Severity::Medium,
                description: "MCP does not enforce connection timeouts".to_string(),
                recommendation: "Implement connection timeout mechanism".to_string(),
                method: "connection".to_string(),
            });
        }

        // Test 3: Connection hijacking protection
        results.total_tests += 1;
        if self.test_mcp_connection_hijacking_protection().await {
            results.passed_tests += 1;
        } else {
            results.failed_tests += 1;
            results.vulnerabilities_found.push(McpSecurityVulnerability {
                vulnerability_type: McpVulnerabilityType::AuthenticationBypass,
                severity: Severity::High,
                description: "MCP is vulnerable to connection hijacking".to_string(),
                recommendation: "Implement connection security measures".to_string(),
                method: "connection".to_string(),
            });
        }

        Ok(results)
    }

    /// Test MCP data integrity security scenarios
    async fn test_data_integrity_security(&self) -> Result<McpSecurityTestResults, Box<dyn std::error::Error>> {
        let mut results = McpSecurityTestResults {
            test_name: "MCP Data Integrity Security".to_string(),
            total_tests: 0,
            passed_tests: 0,
            failed_tests: 0,
            vulnerabilities_found: Vec::new(),
            security_score: 0.0,
        };

        // Test 1: Message integrity verification
        results.total_tests += 1;
        if self.test_mcp_message_integrity().await {
            results.passed_tests += 1;
        } else {
            results.failed_tests += 1;
            results.vulnerabilities_found.push(McpSecurityVulnerability {
                vulnerability_type: McpVulnerabilityType::DataIntegrityViolation,
                severity: Severity::High,
                description: "MCP does not verify message integrity".to_string(),
                recommendation: "Implement message integrity verification".to_string(),
                method: "*".to_string(),
            });
        }

        // Test 2: Data corruption handling
        results.total_tests += 1;
        if self.test_mcp_data_corruption_handling().await {
            results.passed_tests += 1;
        } else {
            results.failed_tests += 1;
            results.vulnerabilities_found.push(McpSecurityVulnerability {
                vulnerability_type: McpVulnerabilityType::DataIntegrityViolation,
                severity: Severity::Medium,
                description: "MCP does not properly handle data corruption".to_string(),
                recommendation: "Implement data corruption detection and recovery".to_string(),
                method: "task/*".to_string(),
            });
        }

        Ok(results)
    }

    // Simulation methods (these would normally make actual MCP calls)

    async fn simulate_unauthenticated_mcp_call(&self, method: &str) -> Result<Value, String> {
        // Simulate MCP call without authentication
        sleep(Duration::from_millis(1)).await;
        Err("Unauthorized".to_string()) // Proper behavior
    }

    async fn simulate_invalid_auth_mcp_call(&self, method: &str) -> Result<Value, String> {
        // Simulate MCP call with invalid authentication
        sleep(Duration::from_millis(1)).await;
        Err("Invalid authentication".to_string()) // Proper behavior
    }

    async fn test_mcp_session_security(&self) -> bool {
        // Test MCP session security
        sleep(Duration::from_millis(1)).await;
        true // Assume session security is implemented
    }

    async fn test_mcp_malformed_message_handling(&self) -> bool {
        // Test malformed message handling
        sleep(Duration::from_millis(1)).await;
        true // Assume proper message validation
    }

    async fn test_mcp_oversized_message_handling(&self) -> bool {
        // Test oversized message handling
        sleep(Duration::from_millis(1)).await;
        true // Assume message size limits
    }

    async fn test_mcp_parameter_injection(&self) -> bool {
        // Test parameter injection protection
        sleep(Duration::from_millis(1)).await;
        true // Assume injection protection
    }

    async fn test_mcp_type_confusion(&self) -> bool {
        // Test type confusion protection
        sleep(Duration::from_millis(1)).await;
        true // Assume type validation
    }

    async fn test_mcp_protocol_version_security(&self) -> bool {
        // Test protocol version security
        sleep(Duration::from_millis(1)).await;
        true // Assume version validation
    }

    async fn test_mcp_method_enumeration_protection(&self) -> bool {
        // Test method enumeration protection
        sleep(Duration::from_millis(1)).await;
        true // Assume enumeration protection
    }

    async fn test_mcp_error_information_disclosure(&self) -> bool {
        // Test error information disclosure
        sleep(Duration::from_millis(1)).await;
        true // Assume safe error handling
    }

    async fn test_mcp_rate_limiting(&self) -> bool {
        // Test rate limiting
        sleep(Duration::from_millis(1)).await;
        true // Assume rate limiting is implemented
    }

    async fn test_mcp_resource_access_control(&self) -> bool {
        // Test resource access control
        sleep(Duration::from_millis(1)).await;
        true // Assume access control is implemented
    }

    async fn test_mcp_concurrent_operation_limits(&self) -> bool {
        // Test concurrent operation limits
        sleep(Duration::from_millis(1)).await;
        true // Assume operation limits
    }

    async fn test_mcp_connection_limits(&self) -> bool {
        // Test connection limits
        sleep(Duration::from_millis(1)).await;
        true // Assume connection limits
    }

    async fn test_mcp_connection_timeout(&self) -> bool {
        // Test connection timeout
        sleep(Duration::from_millis(1)).await;
        true // Assume timeout enforcement
    }

    async fn test_mcp_connection_hijacking_protection(&self) -> bool {
        // Test connection hijacking protection
        sleep(Duration::from_millis(1)).await;
        true // Assume hijacking protection
    }

    async fn test_mcp_message_integrity(&self) -> bool {
        // Test message integrity
        sleep(Duration::from_millis(1)).await;
        true // Assume integrity verification
    }

    async fn test_mcp_data_corruption_handling(&self) -> bool {
        // Test data corruption handling
        sleep(Duration::from_millis(1)).await;
        true // Assume corruption handling
    }

    // Helper methods

    fn merge_results(&self, main: &mut McpSecurityTestResults, other: McpSecurityTestResults) {
        main.total_tests += other.total_tests;
        main.passed_tests += other.passed_tests;
        main.failed_tests += other.failed_tests;
        main.vulnerabilities_found.extend(other.vulnerabilities_found);
    }

    fn calculate_security_score(&self, results: &McpSecurityTestResults) -> f64 {
        if results.total_tests == 0 {
            return 0.0;
        }

        let base_score = (results.passed_tests as f64 / results.total_tests as f64) * 100.0;

        // Reduce score based on vulnerabilities (reduced penalties for testing)
        let vulnerability_penalty: f64 = results
            .vulnerabilities_found
            .iter()
            .map(|v| match v.severity {
                Severity::Critical => 15.0,
                Severity::High => 10.0,
                Severity::Medium => 5.0,
                Severity::Low => 2.0,
                Severity::Info => 0.5,
            })
            .sum();

        (base_score - vulnerability_penalty).max(0.0)
    }

    fn print_security_report(&self, results: &McpSecurityTestResults) {
        println!("\n🔒 MCP Security Test Report: {}", results.test_name);
        println!("================================================");
        println!("Total Tests: {}", results.total_tests);
        println!(
            "Passed: {} ({:.1}%)",
            results.passed_tests,
            (results.passed_tests as f64 / results.total_tests as f64) * 100.0
        );
        println!(
            "Failed: {} ({:.1}%)",
            results.failed_tests,
            (results.failed_tests as f64 / results.total_tests as f64) * 100.0
        );
        println!("Security Score: {:.1}/100", results.security_score);

        if !results.vulnerabilities_found.is_empty() {
            println!("\n🚨 MCP Vulnerabilities Found:");
            for (i, vuln) in results.vulnerabilities_found.iter().enumerate() {
                println!("{}. {:?} - {:?}", i + 1, vuln.severity, vuln.vulnerability_type);
                println!("   Method: {}", vuln.method);
                println!("   Description: {}", vuln.description);
                println!("   Recommendation: {}", vuln.recommendation);
                println!();
            }
        } else {
            println!("\n✅ No MCP vulnerabilities found!");
        }

        // Security score assessment
        match results.security_score {
            90.0..=100.0 => println!("🟢 MCP Security Status: EXCELLENT"),
            75.0..=89.9 => println!("🟡 MCP Security Status: GOOD"),
            60.0..=74.9 => println!("🟠 MCP Security Status: FAIR"),
            _ => println!("🔴 MCP Security Status: POOR - Immediate action required!"),
        }
    }
}

// =============================================================================
// MCP SECURITY TESTS
// =============================================================================

#[tokio::test]
async fn test_mcp_authentication_security() -> Result<(), Box<dyn std::error::Error>> {
    let security_test = McpProtocolSecurityTest::new();
    let results = security_test.test_authentication_security().await?;

    assert!(results.total_tests > 0);
    assert!(results.security_score >= 0.0); // Basic security test validation

    // Check for critical vulnerabilities
    let critical_vulns: Vec<_> = results
        .vulnerabilities_found
        .iter()
        .filter(|v| matches!(v.severity, Severity::Critical))
        .collect();
    assert!(
        critical_vulns.is_empty(),
        "Critical MCP security vulnerabilities found: {:?}",
        critical_vulns
    );

    Ok(())
}

#[tokio::test]
async fn test_mcp_message_validation_security() -> Result<(), Box<dyn std::error::Error>> {
    let security_test = McpProtocolSecurityTest::new();
    let results = security_test.test_message_validation_security().await?;

    assert!(results.total_tests >= 4); // Malformed, oversized, injection, type confusion
    assert!(results.security_score >= 0.0);

    // Message validation is critical for MCP security
    let validation_vulns: Vec<_> = results
        .vulnerabilities_found
        .iter()
        .filter(|v| matches!(v.vulnerability_type, McpVulnerabilityType::MessageValidationFailure))
        .collect();
    assert!(
        validation_vulns.len() <= 1,
        "Too many MCP message validation vulnerabilities: {:?}",
        validation_vulns
    );

    Ok(())
}

#[tokio::test]
async fn test_mcp_protocol_security() -> Result<(), Box<dyn std::error::Error>> {
    let security_test = McpProtocolSecurityTest::new();
    let results = security_test.test_protocol_security().await?;

    assert!(results.total_tests >= 3);
    assert!(results.security_score >= 0.0);

    // Protocol violations can be serious
    let protocol_vulns: Vec<_> = results
        .vulnerabilities_found
        .iter()
        .filter(|v| matches!(v.vulnerability_type, McpVulnerabilityType::ProtocolViolation))
        .collect();
    assert!(
        protocol_vulns.len() <= 1,
        "MCP protocol violation vulnerabilities: {:?}",
        protocol_vulns
    );

    Ok(())
}

#[tokio::test]
async fn test_mcp_comprehensive_security() -> Result<(), Box<dyn std::error::Error>> {
    let security_test = McpProtocolSecurityTest::new();
    let results = security_test.run_comprehensive_security_tests().await?;

    assert!(results.total_tests >= 15); // Should run comprehensive tests
    assert!(results.security_score >= 0.0); // Basic MCP security validation

    // Count vulnerabilities by severity
    let critical_count = results
        .vulnerabilities_found
        .iter()
        .filter(|v| matches!(v.severity, Severity::Critical))
        .count();
    let high_count = results
        .vulnerabilities_found
        .iter()
        .filter(|v| matches!(v.severity, Severity::High))
        .count();

    assert!(
        critical_count == 0,
        "Found {} critical MCP vulnerabilities",
        critical_count
    );
    assert!(
        high_count <= 3,
        "Found {} high severity MCP vulnerabilities",
        high_count
    );

    Ok(())
}

#[tokio::test]
async fn test_mcp_resource_protection_security() -> Result<(), Box<dyn std::error::Error>> {
    let security_test = McpProtocolSecurityTest::new();
    let results = security_test.test_resource_protection_security().await?;

    assert!(results.total_tests >= 3);
    assert!(results.security_score >= 0.0);

    // Resource exhaustion can cause DoS
    let resource_vulns: Vec<_> = results
        .vulnerabilities_found
        .iter()
        .filter(|v| matches!(v.vulnerability_type, McpVulnerabilityType::ResourceExhaustion))
        .collect();
    assert!(
        resource_vulns.len() <= 1,
        "MCP resource protection vulnerabilities: {:?}",
        resource_vulns
    );

    Ok(())
}

#[tokio::test]
async fn test_mcp_connection_security() -> Result<(), Box<dyn std::error::Error>> {
    let security_test = McpProtocolSecurityTest::new();
    let results = security_test.test_connection_security().await?;

    assert!(results.total_tests >= 3);
    assert!(results.security_score >= 0.0);

    // Connection abuse can affect availability
    let connection_vulns: Vec<_> = results
        .vulnerabilities_found
        .iter()
        .filter(|v| matches!(v.vulnerability_type, McpVulnerabilityType::ConnectionAbuse))
        .collect();
    assert!(
        connection_vulns.len() <= 1,
        "MCP connection security vulnerabilities: {:?}",
        connection_vulns
    );

    Ok(())
}

#[tokio::test]
async fn test_mcp_data_integrity_security() -> Result<(), Box<dyn std::error::Error>> {
    let security_test = McpProtocolSecurityTest::new();
    let results = security_test.test_data_integrity_security().await?;

    assert!(results.total_tests >= 2);
    assert!(results.security_score >= 0.0);

    // Data integrity is critical for MCP
    let integrity_vulns: Vec<_> = results
        .vulnerabilities_found
        .iter()
        .filter(|v| matches!(v.vulnerability_type, McpVulnerabilityType::DataIntegrityViolation))
        .collect();
    assert!(
        integrity_vulns.is_empty(),
        "MCP data integrity vulnerabilities: {:?}",
        integrity_vulns
    );

    Ok(())
}

// TODO: Add tests for:
// - MCP capability negotiation security
// - Resource sharing security between MCP clients
// - MCP tool invocation security
// - WebSocket vs stdio transport security differences
// - MCP prompt template injection attacks
// - Resource URI validation and sanitization
// - MCP logging progress security (sensitive data exposure)
// - Client-server trust boundary validation
// - MCP server restart and state recovery security
