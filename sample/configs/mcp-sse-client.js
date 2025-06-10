#!/usr/bin/env node

/**
 * MCP SSE Client for connecting Claude Desktop to a running Ratchet SSE MCP server
 * 
 * This script acts as a bridge between Claude Desktop (which expects stdio-based MCP)
 * and a running Ratchet MCP server using Server-Sent Events (SSE) transport.
 */

const { EventSource } = require('eventsource');
const process = require('process');
const crypto = require('crypto');

// Configuration from environment variables
const serverUrl = process.env.RATCHET_SSE_URL || 'http://localhost:8090';
const timeout = parseInt(process.env.RATCHET_TIMEOUT || '30000');
const authToken = process.env.RATCHET_AUTH_TOKEN; // Optional authentication

class RatchetSSEClient {
    constructor() {
        this.sessionId = crypto.randomUUID();
        this.sseUrl = `${serverUrl}/sse/${this.sessionId}`;
        this.messageUrl = `${serverUrl}/message/${this.sessionId}`;
        this.requestId = 1;
        this.pendingRequests = new Map();
        this.connected = false;
    }

    log(message) {
        // Log to stderr to avoid interfering with JSON-RPC on stdout
        console.error(`[MCP-SSE-Client] ${new Date().toISOString()} ${message}`);
    }

    async connect() {
        this.log(`Connecting to Ratchet MCP server at ${this.sseUrl}`);
        
        try {
            // Create SSE connection
            const headers = {};
            if (authToken) {
                headers['Authorization'] = `Bearer ${authToken}`;
            }

            this.eventSource = new EventSource(this.sseUrl, { headers });
            
            this.eventSource.onopen = () => {
                this.connected = true;
                this.log('Connected to Ratchet MCP server');
            };

            this.eventSource.onmessage = (event) => {
                try {
                    const response = JSON.parse(event.data);
                    
                    // Handle response correlation by ID
                    if (response.id && this.pendingRequests.has(response.id)) {
                        this.pendingRequests.get(response.id)(response);
                        this.pendingRequests.delete(response.id);
                    } else {
                        // Unsolicited message (notification) - send to Claude
                        console.log(JSON.stringify(response));
                    }
                } catch (e) {
                    this.log(`Parse error: ${e.message}`);
                }
            };

            this.eventSource.onerror = (error) => {
                this.connected = false;
                this.log(`SSE connection error: ${error.message || 'Unknown error'}`);
                
                // Send error to Claude if we have an active request
                this.pendingRequests.forEach((resolve, id) => {
                    resolve({
                        jsonrpc: '2.0',
                        id: id,
                        error: {
                            code: -32603,
                            message: 'Connection to Ratchet server lost'
                        }
                    });
                });
                this.pendingRequests.clear();
                
                process.exit(1);
            };

            // Handle stdin from Claude Desktop
            process.stdin.setEncoding('utf8');
            process.stdin.on('data', (data) => {
                this.handleClaudeRequest(data.toString().trim());
            });

            process.stdin.on('end', () => {
                this.log('stdin ended, closing connection');
                this.disconnect();
            });

            // Keep process alive
            process.stdin.resume();

        } catch (error) {
            this.log(`Connection failed: ${error.message}`);
            process.exit(1);
        }
    }

    async handleClaudeRequest(data) {
        if (!data.trim()) return;

        try {
            const request = JSON.parse(data);
            
            // Ensure request has an ID
            if (!request.id) {
                request.id = (this.requestId++).toString();
            }

            this.log(`Sending request: ${request.method || 'unknown'} (id: ${request.id})`);

            // Send request to Ratchet server via HTTP POST
            const headers = {
                'Content-Type': 'application/json'
            };
            
            if (authToken) {
                headers['Authorization'] = `Bearer ${authToken}`;
            }

            const controller = new AbortController();
            const timeoutId = setTimeout(() => controller.abort(), timeout);

            try {
                const response = await fetch(this.messageUrl, {
                    method: 'POST',
                    headers: headers,
                    body: JSON.stringify(request),
                    signal: controller.signal
                });

                clearTimeout(timeoutId);

                if (!response.ok) {
                    throw new Error(`HTTP ${response.status}: ${response.statusText}`);
                }

                // Response will come via SSE, so we don't need to handle it here
                
            } catch (error) {
                clearTimeout(timeoutId);
                
                // Send error response directly to Claude
                const errorResponse = {
                    jsonrpc: '2.0',
                    id: request.id,
                    error: {
                        code: error.name === 'AbortError' ? -32603 : -32603,
                        message: error.name === 'AbortError' ? 'Request timeout' : error.message
                    }
                };
                
                console.log(JSON.stringify(errorResponse));
            }

        } catch (parseError) {
            this.log(`Failed to parse request: ${parseError.message}`);
            
            // Send parse error to Claude
            const errorResponse = {
                jsonrpc: '2.0',
                id: null,
                error: {
                    code: -32700,
                    message: 'Parse error'
                }
            };
            
            console.log(JSON.stringify(errorResponse));
        }
    }

    disconnect() {
        if (this.eventSource) {
            this.eventSource.close();
        }
        this.connected = false;
        this.log('Disconnected from Ratchet MCP server');
    }
}

// Handle graceful shutdown
process.on('SIGINT', () => {
    console.error('\nReceived SIGINT, shutting down...');
    process.exit(0);
});

process.on('SIGTERM', () => {
    console.error('\nReceived SIGTERM, shutting down...');
    process.exit(0);
});

// Start the client
const client = new RatchetSSEClient();
client.connect().catch((error) => {
    console.error(`Failed to start MCP SSE client: ${error.message}`);
    process.exit(1);
});