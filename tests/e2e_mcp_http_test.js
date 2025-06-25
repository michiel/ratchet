#!/usr/bin/env node

/**
 * End-to-End Test for Ratchet MCP HTTP Interface
 * 
 * This test verifies the complete MCP HTTP workflow:
 * 1. Start ratchet server process
 * 2. Connect via HTTP MCP interface
 * 3. Create a new task (multiply function)
 * 4. Execute the task with test data
 * 5. Inspect execution results and logs
 * 6. Debug the task execution
 * 7. Verify return values
 * 
 * Tests ONLY the HTTP MCP interface - no direct API calls.
 */

const { spawn } = require('child_process');
const fs = require('fs').promises;
const path = require('path');
const axios = require('axios');

class McpHttpClient {
    constructor(baseUrl = 'http://localhost:8081') {
        this.baseUrl = baseUrl;
        this.sessionId = null;
        this.requestId = 1;
    }

    async request(method, params = {}) {
        const payload = {
            jsonrpc: '2.0',
            id: this.requestId++,
            method: method,
            params: params
        };

        try {
            const response = await axios.post(`${this.baseUrl}/mcp`, payload, {
                headers: {
                    'Content-Type': 'application/json'
                },
                timeout: 30000
            });

            if (response.data.error) {
                throw new Error(`MCP Error: ${response.data.error.message}`);
            }

            return response.data.result;
        } catch (error) {
            if (error.response) {
                throw new Error(`HTTP ${error.response.status}: ${error.response.statusText}`);
            }
            throw error;
        }
    }

    async toolCall(toolName, args = {}) {
        return await this.request('tools/call', {
            name: toolName,
            arguments: args
        });
    }

    async listTools() {
        return await this.request('tools/list');
    }

    async initialize() {
        return await this.request('initialize', {
            protocolVersion: '2024-11-05',
            capabilities: {
                tools: {}
            },
            clientInfo: {
                name: 'e2e-test-client',
                version: '1.0.0'
            }
        });
    }
}

class RatchetServerManager {
    constructor() {
        this.process = null;
        this.configPath = null;
    }

    async createTestConfig() {
        const testConfig = {
            server: {
                host: '127.0.0.1',
                port: 8081,
                workers: 2,
                database: {
                    url: 'sqlite://e2e_test.db'
                }
            },
            rest_api: {
                enabled: true,
                prefix: '/api/v1'
            },
            logging: {
                level: 'debug',
                targets: [
                    {
                        type: 'console',
                        enabled: true
                    }
                ]
            },
            registry: {
                update_interval: 60,
                concurrent_updates: 2,
                retry_attempts: 3,
                sources: [
                    {
                        name: 'test-tasks',
                        polling_interval: 30,
                        uri: 'file://test-tasks',
                        config: {
                            watch_for_changes: true,
                            auto_reload: true,
                            include_patterns: ['*.js', '*.yaml', '*.json'],
                            recursive: true,
                            max_depth: 5
                        }
                    }
                ]
            },
            mcp: {
                enabled: true,
                transport: 'sse',
                server: {
                    host: '127.0.0.1',
                    port: 8091,
                    enable_cors: true
                },
                tools: {
                    enable_execution: true,
                    enable_logging: true,
                    enable_debugging: true
                }
            }
        };

        this.configPath = path.join(__dirname, 'e2e-test-config.yaml');
        const yaml = require('js-yaml');
        await fs.writeFile(this.configPath, yaml.dump(testConfig));
        return this.configPath;
    }

    async start() {
        await this.createTestConfig();

        return new Promise((resolve, reject) => {
            const ratchetBinary = path.join(__dirname, '../target/debug/ratchet');
            
            this.process = spawn(ratchetBinary, ['serve', '--config', this.configPath], {
                stdio: ['pipe', 'pipe', 'pipe'],
                env: { ...process.env, RUST_LOG: 'debug' }
            });

            let startupOutput = '';
            
            this.process.stdout.on('data', (data) => {
                const output = data.toString();
                startupOutput += output;
                console.log(`[RATCHET] ${output.trim()}`);
                
                // Look for startup success indicators
                if (output.includes('Server started') || output.includes('listening on') || 
                    output.includes('Starting HTTP server on') || output.includes('✅ =====')) {
                    setTimeout(() => resolve(), 3000); // Give it time to fully start
                }
            });

            this.process.stderr.on('data', (data) => {
                const output = data.toString();
                console.error(`[RATCHET ERROR] ${output.trim()}`);
                
                // Don't reject on stderr - some logs go there
                if (output.includes('Error') || output.includes('Failed')) {
                    // Only reject if it's clearly a startup failure
                    if (startupOutput.length < 100) { // Likely failed immediately
                        reject(new Error(`Ratchet startup failed: ${output}`));
                    }
                }
            });

            this.process.on('error', (error) => {
                reject(new Error(`Failed to start ratchet process: ${error.message}`));
            });

            this.process.on('exit', (code) => {
                if (code !== 0) {
                    reject(new Error(`Ratchet process exited with code ${code}`));
                }
            });

            // Timeout after 15 seconds
            setTimeout(() => {
                reject(new Error('Ratchet server startup timeout'));
            }, 15000);
        });
    }

    async stop() {
        if (this.process) {
            this.process.kill('SIGTERM');
            
            // Wait for graceful shutdown
            await new Promise((resolve) => {
                this.process.on('exit', resolve);
                setTimeout(() => {
                    this.process.kill('SIGKILL');
                    resolve();
                }, 5000);
            });
        }

        // Cleanup config file
        if (this.configPath) {
            try {
                await fs.unlink(this.configPath);
            } catch (error) {
                // Ignore cleanup errors
            }
        }

        // Cleanup test database
        try {
            await fs.unlink('e2e_test.db');
        } catch (error) {
            // Ignore cleanup errors
        }
    }
}

class E2ETestRunner {
    constructor() {
        this.serverManager = new RatchetServerManager();
        this.mcpClient = new McpHttpClient();
        this.testResults = [];
        this.capabilityGaps = [];
    }

    log(message, level = 'INFO') {
        const timestamp = new Date().toISOString();
        console.log(`[${timestamp}] [${level}] ${message}`);
    }

    logCapabilityGap(gap) {
        this.capabilityGaps.push(gap);
        this.log(`🚨 CAPABILITY GAP: ${gap}`, 'WARN');
    }

    async test(name, testFn) {
        this.log(`🧪 Starting test: ${name}`);
        const startTime = Date.now();
        
        try {
            await testFn();
            const duration = Date.now() - startTime;
            this.testResults.push({ name, status: 'PASS', duration });
            this.log(`✅ Test passed: ${name} (${duration}ms)`, 'SUCCESS');
        } catch (error) {
            const duration = Date.now() - startTime;
            this.testResults.push({ name, status: 'FAIL', duration, error: error.message });
            this.log(`❌ Test failed: ${name} (${duration}ms): ${error.message}`, 'ERROR');
            throw error;
        }
    }

    async waitForServer(maxAttempts = 30) {
        this.log('Waiting for MCP server to be ready...');
        
        for (let attempt = 1; attempt <= maxAttempts; attempt++) {
            try {
                await this.mcpClient.initialize();
                this.log('MCP server is ready!');
                return;
            } catch (error) {
                if (attempt === maxAttempts) {
                    throw new Error(`MCP server not ready after ${maxAttempts} attempts: ${error.message}`);
                }
                await new Promise(resolve => setTimeout(resolve, 1000));
            }
        }
    }

    async runTests() {
        this.log('🚀 Starting E2E MCP HTTP Test Suite');

        try {
            // Start server
            await this.test('Server Startup', async () => {
                await this.serverManager.start();
            });

            // Wait for server to be ready
            await this.test('Server Readiness', async () => {
                await this.waitForServer();
            });

            // MCP connection was tested in waitForServer

            // List available tools
            await this.test('Tool Discovery', async () => {
                const tools = await this.mcpClient.listTools();
                this.log(`Found ${tools.tools.length} available tools`);
                
                const requiredTools = [
                    'ratchet.create_task',
                    'ratchet.execute_task', 
                    'ratchet.get_execution_logs',
                    'ratchet.debug_task_execution'
                ];

                for (const tool of requiredTools) {
                    if (!tools.tools.find(t => t.name === tool)) {
                        this.logCapabilityGap(`Missing required tool: ${tool}`);
                    }
                }
            });

            // Create multiply task
            let createdTaskId = null;
            await this.test('Task Creation', async () => {
                const multiplyTaskCode = `
function multiply(input) {
    const { a, b } = input;
    
    if (typeof a !== 'number' || typeof b !== 'number') {
        throw new Error('Both a and b must be numbers');
    }
    
    const result = a * b;
    
    return {
        result: result,
        calculation: \`\${a} × \${b} = \${result}\`,
        timestamp: new Date().toISOString()
    };
}

// Export for Ratchet
if (typeof module !== 'undefined' && module.exports) {
    module.exports = { multiply };
}
`;

                const inputSchema = {
                    type: 'object',
                    properties: {
                        a: { type: 'number', description: 'First number to multiply' },
                        b: { type: 'number', description: 'Second number to multiply' }
                    },
                    required: ['a', 'b']
                };

                const outputSchema = {
                    type: 'object',
                    properties: {
                        result: { type: 'number', description: 'The multiplication result' },
                        calculation: { type: 'string', description: 'Human-readable calculation' },
                        timestamp: { type: 'string', description: 'When the calculation was performed' }
                    },
                    required: ['result', 'calculation', 'timestamp']
                };

                try {
                    const result = await this.mcpClient.toolCall('ratchet.create_task', {
                        name: 'multiply',
                        description: 'Multiplies two numbers and returns the result with metadata',
                        code: multiplyTaskCode,
                        input_schema: inputSchema,
                        output_schema: outputSchema,
                        version: '1.0.0',
                        test_cases: [
                            {
                                name: 'basic_multiplication',
                                input: { a: 6, b: 7 },
                                expected_output: { result: 42 }
                            },
                            {
                                name: 'multiply_by_zero',
                                input: { a: 5, b: 0 },
                                expected_output: { result: 0 }
                            }
                        ]
                    });

                    if (result.content && result.content[0] && result.content[0].text) {
                        const taskInfo = JSON.parse(result.content[0].text);
                        createdTaskId = taskInfo.task_id || taskInfo.uuid || taskInfo.id;
                        this.log(`Created task with ID: ${createdTaskId}`);
                    } else {
                        throw new Error('Task creation response missing expected content');
                    }
                } catch (error) {
                    this.logCapabilityGap(`Task creation failed: ${error.message}`);
                    throw error;
                }
            });

            // Execute the task
            let executionId = null;
            await this.test('Task Execution', async () => {
                if (!createdTaskId) {
                    throw new Error('No task ID from creation step');
                }

                try {
                    const result = await this.mcpClient.toolCall('ratchet.execute_task', {
                        task_id: createdTaskId,
                        input: { a: 8, b: 9 },
                        trace: true
                    });

                    if (result.content && result.content[0] && result.content[0].text) {
                        const output = JSON.parse(result.content[0].text);
                        
                        if (output.result === 72 && output.calculation === '8 × 9 = 72') {
                            this.log(`✅ Task execution successful: ${output.calculation}`);
                        } else {
                            throw new Error(`Unexpected task output: ${JSON.stringify(output)}`);
                        }

                        // Check for execution ID in metadata
                        if (result.metadata && result.metadata.execution_id) {
                            executionId = result.metadata.execution_id;
                        }
                    } else {
                        throw new Error('Task execution response missing expected content');
                    }
                } catch (error) {
                    this.logCapabilityGap(`Task execution failed: ${error.message}`);
                    throw error;
                }
            });

            // Test streaming execution
            await this.test('Streaming Execution', async () => {
                if (!createdTaskId) {
                    throw new Error('No task ID from creation step');
                }

                try {
                    const result = await this.mcpClient.toolCall('ratchet.execute_task', {
                        task_id: createdTaskId,
                        input: { a: 12, b: 5 },
                        stream_progress: true,
                        trace: true
                    });

                    if (result.content && result.content[0] && result.content[0].text) {
                        const response = JSON.parse(result.content[0].text);
                        
                        if (response.execution_id && response.streaming === true) {
                            this.log(`✅ Streaming execution started: ${response.execution_id}`);
                            executionId = response.execution_id;
                        } else {
                            throw new Error(`Unexpected streaming response: ${JSON.stringify(response)}`);
                        }
                    } else {
                        throw new Error('Streaming execution response missing expected content');
                    }
                } catch (error) {
                    this.logCapabilityGap(`Streaming execution failed: ${error.message}`);
                    throw error;
                }
            });

            // Get execution logs
            await this.test('Execution Logs', async () => {
                if (!executionId) {
                    this.logCapabilityGap('No execution ID available for log retrieval');
                    return;
                }

                try {
                    const result = await this.mcpClient.toolCall('ratchet.get_execution_logs', {
                        execution_id: executionId,
                        format: 'json',
                        limit: 100
                    });

                    if (result.content && result.content[0] && result.content[0].text) {
                        const logs = JSON.parse(result.content[0].text);
                        this.log(`✅ Retrieved ${logs.logs ? logs.logs.length : 0} log entries`);
                    } else {
                        this.logCapabilityGap('Execution logs response missing expected content');
                    }
                } catch (error) {
                    this.logCapabilityGap(`Execution log retrieval failed: ${error.message}`);
                }
            });

            // Test debugging capabilities
            await this.test('Task Debugging', async () => {
                if (!createdTaskId) {
                    throw new Error('No task ID from creation step');
                }

                try {
                    const result = await this.mcpClient.toolCall('ratchet.debug_task_execution', {
                        task_id: createdTaskId,
                        input: { a: 3, b: 4 },
                        capture_variables: true,
                        step_mode: false,
                        timeout_ms: 30000
                    });

                    if (result.content && result.content[0] && result.content[0].text) {
                        const debugInfo = JSON.parse(result.content[0].text);
                        this.log(`✅ Task debugging completed with ${debugInfo.steps ? debugInfo.steps.length : 0} debug steps`);
                    } else {
                        this.logCapabilityGap('Task debugging response missing expected content');
                    }
                } catch (error) {
                    this.logCapabilityGap(`Task debugging failed: ${error.message}`);
                }
            });

            // Test execution inspection
            await this.test('Execution Inspection', async () => {
                try {
                    const result = await this.mcpClient.toolCall('ratchet.list_executions', {
                        task_id: createdTaskId,
                        limit: 10,
                        include_output: true
                    });

                    if (result.content && result.content[0] && result.content[0].text) {
                        const executions = JSON.parse(result.content[0].text);
                        this.log(`✅ Found ${executions.executions ? executions.executions.length : 0} executions for task`);
                        
                        // Verify we can find our executions
                        if (executions.executions && executions.executions.length > 0) {
                            const execution = executions.executions[0];
                            if (execution.output && execution.output.result) {
                                this.log(`✅ Execution output verified: result = ${execution.output.result}`);
                            }
                        }
                    } else {
                        this.logCapabilityGap('Execution inspection response missing expected content');
                    }
                } catch (error) {
                    this.logCapabilityGap(`Execution inspection failed: ${error.message}`);
                }
            });

            // Test task validation
            await this.test('Task Validation', async () => {
                if (!createdTaskId) {
                    throw new Error('No task ID from creation step');
                }

                try {
                    const result = await this.mcpClient.toolCall('ratchet.validate_task', {
                        task_id: createdTaskId,
                        run_tests: true,
                        syntax_only: false
                    });

                    if (result.content && result.content[0] && result.content[0].text) {
                        const validation = JSON.parse(result.content[0].text);
                        this.log(`✅ Task validation completed: ${validation.valid ? 'VALID' : 'INVALID'}`);
                    } else {
                        this.logCapabilityGap('Task validation response missing expected content');
                    }
                } catch (error) {
                    this.logCapabilityGap(`Task validation failed: ${error.message}`);
                }
            });

            this.log('🎉 All tests completed successfully!');

        } catch (error) {
            this.log(`💥 Test suite failed: ${error.message}`, 'ERROR');
            throw error;
        } finally {
            // Always cleanup
            await this.serverManager.stop();
        }
    }

    printSummary() {
        this.log('\n📊 Test Summary:');
        this.log('================');
        
        const passed = this.testResults.filter(t => t.status === 'PASS').length;
        const failed = this.testResults.filter(t => t.status === 'FAIL').length;
        const totalTime = this.testResults.reduce((sum, t) => sum + t.duration, 0);
        
        this.log(`Total Tests: ${this.testResults.length}`);
        this.log(`Passed: ${passed}`);
        this.log(`Failed: ${failed}`);
        this.log(`Total Time: ${totalTime}ms`);
        
        if (this.capabilityGaps.length > 0) {
            this.log('\n🚨 Capability Gaps Identified:');
            this.log('==============================');
            this.capabilityGaps.forEach((gap, index) => {
                this.log(`${index + 1}. ${gap}`);
            });
        } else {
            this.log('\n✅ No capability gaps identified!');
        }
        
        this.log('\n📋 Detailed Results:');
        this.log('====================');
        this.testResults.forEach(result => {
            const status = result.status === 'PASS' ? '✅' : '❌';
            this.log(`${status} ${result.name} (${result.duration}ms)`);
            if (result.error) {
                this.log(`   Error: ${result.error}`);
            }
        });
    }
}

// Run the test suite
async function main() {
    const runner = new E2ETestRunner();
    
    try {
        await runner.runTests();
        runner.printSummary();
        process.exit(0);
    } catch (error) {
        runner.printSummary();
        console.error('\n💥 Test suite failed:', error.message);
        process.exit(1);
    }
}

if (require.main === module) {
    main().catch(console.error);
}

module.exports = { E2ETestRunner, McpHttpClient, RatchetServerManager };