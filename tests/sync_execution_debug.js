#!/usr/bin/env node

/**
 * Debug Synchronous vs Streaming Execution Issue
 * 
 * This test specifically investigates why synchronous execution returns null
 * while streaming execution appears to work.
 */

const axios = require('axios');

class SyncExecutionDebugger {
    constructor() {
        this.baseUrl = 'http://localhost:8080/mcp';
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

            return response.data;
        } catch (error) {
            throw error;
        }
    }

    async toolCall(toolName, args = {}) {
        const result = await this.request('tools/call', {
            name: toolName,
            arguments: args
        });
        
        return result.result;
    }

    async debug() {
        console.log('🔍 Debugging Synchronous vs Streaming Execution\n');

        try {
            // First, let's test with a task that we know exists
            console.log('1. Testing with heartbeat task (built-in)...');
            
            // Test sync execution with heartbeat
            console.log('\n📍 Testing SYNCHRONOUS execution with heartbeat:');
            try {
                const syncResult = await this.toolCall('ratchet_execute_task', {
                    task_id: 'heartbeat',
                    input: {}
                });
                console.log('✅ Sync heartbeat result:');
                console.log(JSON.stringify(syncResult, null, 2));
            } catch (error) {
                console.log('❌ Sync heartbeat failed:');
                console.log(`   Error: ${error.message}`);
                if (error.response?.data) {
                    console.log('   Response:', JSON.stringify(error.response.data, null, 2));
                }
            }

            // Test streaming execution with heartbeat
            console.log('\n📍 Testing STREAMING execution with heartbeat:');
            try {
                const streamResult = await this.toolCall('ratchet_execute_task', {
                    task_id: 'heartbeat',
                    input: {},
                    stream_progress: true
                });
                console.log('✅ Stream heartbeat result:');
                console.log(JSON.stringify(streamResult, null, 2));
            } catch (error) {
                console.log('❌ Stream heartbeat failed:');
                console.log(`   Error: ${error.message}`);
                if (error.response?.data) {
                    console.log('   Response:', JSON.stringify(error.response.data, null, 2));
                }
            }

            // Test with available tasks
            console.log('\n2. Checking available tasks...');
            try {
                const tasks = await this.toolCall('ratchet_list_available_tasks', {
                    limit: 5
                });
                console.log('📋 Available tasks:');
                console.log(JSON.stringify(tasks, null, 2));

                if (tasks.tasks && tasks.tasks.length > 0) {
                    const firstTask = tasks.tasks[0];
                    console.log(`\n3. Testing with available task: ${firstTask.name}`);

                    // Test sync with first available task
                    console.log('\n📍 Testing SYNCHRONOUS execution:');
                    try {
                        const syncResult = await this.toolCall('ratchet_execute_task', {
                            task_id: firstTask.name,
                            input: {}
                        });
                        console.log('✅ Sync result:');
                        console.log(JSON.stringify(syncResult, null, 2));
                    } catch (error) {
                        console.log('❌ Sync failed:');
                        console.log(`   Error: ${error.message}`);
                        if (error.response?.data) {
                            console.log('   Full response:', JSON.stringify(error.response.data, null, 2));
                        }
                    }

                    // Test streaming with first available task
                    console.log('\n📍 Testing STREAMING execution:');
                    try {
                        const streamResult = await this.toolCall('ratchet_execute_task', {
                            task_id: firstTask.name,
                            input: {},
                            stream_progress: true
                        });
                        console.log('✅ Stream result:');
                        console.log(JSON.stringify(streamResult, null, 2));
                    } catch (error) {
                        console.log('❌ Stream failed:');
                        console.log(`   Error: ${error.message}`);
                        if (error.response?.data) {
                            console.log('   Full response:', JSON.stringify(error.response.data, null, 2));
                        }
                    }
                }

            } catch (error) {
                console.log('❌ Could not list tasks:');
                console.log(`   Error: ${error.message}`);
            }

            console.log('\n🔍 Analysis Complete');

        } catch (error) {
            console.error(`💥 Debug failed: ${error.message}`);
        }
    }
}

// Run the debugger
async function main() {
    console.log('Waiting for server to be ready...');
    await new Promise(resolve => setTimeout(resolve, 2000));
    
    const debugInstance = new SyncExecutionDebugger();
    await debugInstance.debug();
}

main().catch(console.error);