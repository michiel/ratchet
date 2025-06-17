/**
 * Ratchet Heartbeat Task
 * 
 * Built-in system heartbeat task that provides health status information.
 * This task is embedded in the Ratchet binary and runs automatically
 * every 5 minutes to ensure system health monitoring.
 */

async function main(input) {
    const startTime = Date.now();
    
    try {
        // Basic system status
        const timestamp = new Date().toISOString();
        
        // Get process uptime (Node.js specific)
        const uptimeSeconds = process.uptime();
        
        // Basic system information
        const systemInfo = {
            version: process.env.RATCHET_VERSION || "unknown",
            uptime_seconds: Math.floor(uptimeSeconds),
            active_jobs: 0 // This would be populated by the execution context
        };
        
        // Calculate execution time
        const executionTime = Date.now() - startTime;
        
        // Return success response
        return {
            status: "ok",
            timestamp: timestamp,
            message: `Heartbeat successful - system running normally (${executionTime}ms)`,
            system_info: systemInfo
        };
        
    } catch (error) {
        // Return error response if something goes wrong
        return {
            status: "error",
            timestamp: new Date().toISOString(),
            message: `Heartbeat failed: ${error.message}`,
            system_info: {
                version: process.env.RATCHET_VERSION || "unknown",
                uptime_seconds: process.uptime ? Math.floor(process.uptime()) : 0,
                active_jobs: 0
            }
        };
    }
}

// Export for CommonJS compatibility
if (typeof module !== 'undefined' && module.exports) {
    module.exports = { main };
}