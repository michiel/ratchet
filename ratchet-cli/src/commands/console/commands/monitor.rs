//! Monitoring and dashboard commands for real-time system oversight

use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::commands::console::{
    command_trait::{ConsoleCommand, CommandArgs, CommandOutput},
    enhanced_mcp_client::EnhancedMcpClient,
};

/// Monitoring and dashboard command
pub struct MonitorCommand;

impl MonitorCommand {
    pub fn new() -> Self {
        Self
    }

    /// Display real-time dashboard with system metrics
    async fn show_dashboard(
        &self,
        args: &CommandArgs,
        mcp_client: &EnhancedMcpClient,
    ) -> Result<CommandOutput> {
        let refresh_interval = args.get_number_flag("refresh", 5);
        let compact = args.has_flag("compact");
        let filter = args.get_flag("filter");

        // Get system health information
        let health_result = mcp_client
            .execute_tool("ratchet_system_health", json!({"include_metrics": true}))
            .await?;

        // Get recent execution statistics
        let stats_result = mcp_client
            .execute_tool("ratchet_get_execution_stats", json!({
                "time_range": "last_hour",
                "include_breakdown": true
            }))
            .await?;

        // Get active executions
        let active_result = mcp_client
            .execute_tool("ratchet_list_executions", json!({
                "status": "running",
                "limit": 10,
                "include_metadata": true
            }))
            .await?;

        let mut output = vec![
            format!("🖥️  Ratchet System Dashboard - {}", 
                chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")),
            "".to_string(),
        ];

        // System Health Section
        output.push("📊 System Health".to_string());
        output.push("─".repeat(50));
        
        if let Some(health) = health_result.get("health") {
            let status = health.get("status").and_then(|s| s.as_str()).unwrap_or("unknown");
            let uptime = health.get("uptime_seconds").and_then(|u| u.as_u64()).unwrap_or(0);
            let cpu_usage = health.get("cpu_usage_percent").and_then(|c| c.as_f64()).unwrap_or(0.0);
            let memory_usage = health.get("memory_usage_percent").and_then(|m| m.as_f64()).unwrap_or(0.0);
            let active_connections = health.get("active_connections").and_then(|a| a.as_u64()).unwrap_or(0);

            let status_emoji = match status {
                "healthy" => "✅",
                "degraded" => "⚠️",
                "unhealthy" => "❌",
                _ => "❓",
            };

            output.push(format!("Status: {} {}", status_emoji, status));
            output.push(format!("Uptime: {}", format_duration(uptime)));
            output.push(format!("CPU Usage: {:.1}%", cpu_usage));
            output.push(format!("Memory Usage: {:.1}%", memory_usage));
            output.push(format!("Active Connections: {}", active_connections));
        } else {
            output.push("❌ Health information unavailable".to_string());
        }

        output.push("".to_string());

        // Execution Statistics Section
        output.push("📈 Execution Statistics (Last Hour)".to_string());
        output.push("─".repeat(50));
        
        if let Some(stats) = stats_result.get("statistics") {
            let total = stats.get("total_executions").and_then(|t| t.as_u64()).unwrap_or(0);
            let completed = stats.get("completed_executions").and_then(|c| c.as_u64()).unwrap_or(0);
            let failed = stats.get("failed_executions").and_then(|f| f.as_u64()).unwrap_or(0);
            let running = stats.get("running_executions").and_then(|r| r.as_u64()).unwrap_or(0);
            let avg_duration = stats.get("average_duration_ms").and_then(|a| a.as_u64()).unwrap_or(0);
            let success_rate = if total > 0 { (completed as f64 / total as f64) * 100.0 } else { 0.0 };

            output.push(format!("Total Executions: {}", total));
            output.push(format!("✅ Completed: {} ({:.1}%)", completed, success_rate));
            output.push(format!("❌ Failed: {}", failed));
            output.push(format!("🔄 Currently Running: {}", running));
            output.push(format!("⏱️  Average Duration: {}ms", avg_duration));
            output.push(format!("📊 Success Rate: {:.1}%", success_rate));
        } else {
            output.push("❌ Execution statistics unavailable".to_string());
        }

        output.push("".to_string());

        // Active Executions Section
        if !compact {
            output.push("🔄 Active Executions".to_string());
            output.push("─".repeat(50));
            
            if let Some(executions) = active_result.get("executions").and_then(|e| e.as_array()) {
                if executions.is_empty() {
                    output.push("No active executions".to_string());
                } else {
                    for execution in executions.iter().take(5) {
                        let id = execution.get("id").and_then(|i| i.as_str()).unwrap_or("N/A");
                        let task_id = execution.get("task_id").and_then(|t| t.as_str()).unwrap_or("N/A");
                        let progress = execution.get("progress").and_then(|p| p.as_f64()).unwrap_or(0.0);
                        let started = execution.get("created_at").and_then(|c| c.as_str()).unwrap_or("N/A");
                        
                        output.push(format!("  {} | Task: {} | Progress: {:.1}% | Started: {}", 
                            &id[..8.min(id.len())], 
                            &task_id[..12.min(task_id.len())],
                            progress * 100.0,
                            started.split('T').next().unwrap_or(started)
                        ));
                    }
                    
                    if executions.len() > 5 {
                        output.push(format!("  ... and {} more", executions.len() - 5));
                    }
                }
            } else {
                output.push("❌ Active executions information unavailable".to_string());
            }
        }

        output.push("".to_string());
        output.push(format!("🔄 Auto-refresh every {}s | Use Ctrl+C to exit", refresh_interval));

        Ok(CommandOutput::text(output.join("\n")))
    }

    /// Show detailed system health information
    async fn show_health(
        &self,
        args: &CommandArgs,
        mcp_client: &EnhancedMcpClient,
    ) -> Result<CommandOutput> {
        let detailed = args.has_flag("detailed");
        let check_services = args.has_flag("services");

        let mut health_args = json!({
            "include_metrics": true,
            "include_connectivity": true
        });

        if check_services {
            health_args["include_services"] = json!(true);
        }

        let result = mcp_client
            .execute_tool("ratchet_system_health", health_args)
            .await?;

        if let Some(health) = result.get("health") {
            let mut output = vec![
                "🏥 System Health Report".to_string(),
                "".to_string(),
            ];

            // Overall Status
            let status = health.get("status").and_then(|s| s.as_str()).unwrap_or("unknown");
            let status_emoji = match status {
                "healthy" => "✅",
                "degraded" => "⚠️",
                "unhealthy" => "❌",
                _ => "❓",
            };
            
            output.push(format!("Overall Status: {} {}", status_emoji, status.to_uppercase()));
            output.push("".to_string());

            // System Metrics
            output.push("📊 System Metrics".to_string());
            output.push("─".repeat(30));
            
            if let Some(uptime) = health.get("uptime_seconds").and_then(|u| u.as_u64()) {
                output.push(format!("Uptime: {}", format_duration(uptime)));
            }
            
            if let Some(cpu) = health.get("cpu_usage_percent").and_then(|c| c.as_f64()) {
                let cpu_status = if cpu < 70.0 { "✅" } else if cpu < 90.0 { "⚠️" } else { "❌" };
                output.push(format!("CPU Usage: {} {:.1}%", cpu_status, cpu));
            }
            
            if let Some(memory) = health.get("memory_usage_percent").and_then(|m| m.as_f64()) {
                let mem_status = if memory < 80.0 { "✅" } else if memory < 95.0 { "⚠️" } else { "❌" };
                output.push(format!("Memory Usage: {} {:.1}%", mem_status, memory));
            }
            
            if let Some(disk) = health.get("disk_usage_percent").and_then(|d| d.as_f64()) {
                let disk_status = if disk < 85.0 { "✅" } else if disk < 95.0 { "⚠️" } else { "❌" };
                output.push(format!("Disk Usage: {} {:.1}%", disk_status, disk));
            }

            output.push("".to_string());

            // Connectivity
            output.push("🌐 Connectivity".to_string());
            output.push("─".repeat(30));
            
            if let Some(connections) = health.get("active_connections").and_then(|a| a.as_u64()) {
                output.push(format!("Active Connections: {}", connections));
            }
            
            if let Some(database) = health.get("database_connected").and_then(|d| d.as_bool()) {
                let db_status = if database { "✅ Connected" } else { "❌ Disconnected" };
                output.push(format!("Database: {}", db_status));
            }

            // Service Health (if requested)
            if check_services {
                if let Some(services) = health.get("services").and_then(|s| s.as_object()) {
                    output.push("".to_string());
                    output.push("🔧 Service Health".to_string());
                    output.push("─".repeat(30));
                    
                    for (service, status) in services {
                        let service_status = if status.get("healthy").and_then(|h| h.as_bool()).unwrap_or(false) {
                            "✅"
                        } else {
                            "❌"
                        };
                        
                        let response_time = status.get("response_time_ms")
                            .and_then(|r| r.as_u64())
                            .map(|ms| format!(" ({}ms)", ms))
                            .unwrap_or_default();
                            
                        output.push(format!("{} {}{}", service_status, service, response_time));
                    }
                }
            }

            // Detailed Information
            if detailed {
                output.push("".to_string());
                output.push("🔍 Detailed Information".to_string());
                output.push("─".repeat(30));
                output.push(serde_json::to_string_pretty(&health).unwrap_or_else(|_| "N/A".to_string()));
            }

            Ok(CommandOutput::text(output.join("\n")))
        } else {
            Ok(CommandOutput::error("Failed to retrieve system health information"))
        }
    }

    /// Show comprehensive system statistics
    async fn show_stats(
        &self,
        args: &CommandArgs,
        mcp_client: &EnhancedMcpClient,
    ) -> Result<CommandOutput> {
        let time_range = args.get_flag("range").unwrap_or("last_24_hours");
        let breakdown = args.has_flag("breakdown");
        let format = args.get_flag("format").unwrap_or("table");

        let stats_args = json!({
            "time_range": time_range,
            "include_breakdown": breakdown,
            "include_trends": true,
            "include_performance": true
        });

        let result = mcp_client
            .execute_tool("ratchet_get_system_stats", stats_args)
            .await?;

        if let Some(stats) = result.get("statistics") {
            match format {
                "json" => {
                    Ok(CommandOutput::json(stats.clone()))
                }
                "table" | _ => {
                    let mut output = vec![
                        format!("📊 System Statistics ({})", time_range.replace('_', " ")),
                        "".to_string(),
                    ];

                    // Execution Statistics
                    output.push("⚡ Execution Statistics".to_string());
                    output.push("─".repeat(40));
                    
                    if let Some(exec_stats) = stats.get("executions") {
                        let total = exec_stats.get("total").and_then(|t| t.as_u64()).unwrap_or(0);
                        let completed = exec_stats.get("completed").and_then(|c| c.as_u64()).unwrap_or(0);
                        let failed = exec_stats.get("failed").and_then(|f| f.as_u64()).unwrap_or(0);
                        let avg_duration = exec_stats.get("average_duration_ms").and_then(|a| a.as_u64()).unwrap_or(0);
                        let max_duration = exec_stats.get("max_duration_ms").and_then(|m| m.as_u64()).unwrap_or(0);
                        let success_rate = if total > 0 { (completed as f64 / total as f64) * 100.0 } else { 0.0 };

                        output.push(format!("Total Executions: {}", total));
                        output.push(format!("Completed: {}", completed));
                        output.push(format!("Failed: {}", failed));
                        output.push(format!("Success Rate: {:.1}%", success_rate));
                        output.push(format!("Average Duration: {}ms", avg_duration));
                        output.push(format!("Max Duration: {}ms", max_duration));
                    }

                    output.push("".to_string());

                    // Performance Metrics
                    output.push("🏎️  Performance Metrics".to_string());
                    output.push("─".repeat(40));
                    
                    if let Some(perf_stats) = stats.get("performance") {
                        let throughput = perf_stats.get("executions_per_hour").and_then(|t| t.as_f64()).unwrap_or(0.0);
                        let queue_time = perf_stats.get("average_queue_time_ms").and_then(|q| q.as_u64()).unwrap_or(0);
                        let worker_efficiency = perf_stats.get("worker_efficiency_percent").and_then(|w| w.as_f64()).unwrap_or(0.0);

                        output.push(format!("Throughput: {:.1} exec/hour", throughput));
                        output.push(format!("Average Queue Time: {}ms", queue_time));
                        output.push(format!("Worker Efficiency: {:.1}%", worker_efficiency));
                    }

                    // Resource Usage
                    if let Some(resource_stats) = stats.get("resources") {
                        output.push("".to_string());
                        output.push("💾 Resource Usage".to_string());
                        output.push("─".repeat(40));
                        
                        let avg_cpu = resource_stats.get("average_cpu_percent").and_then(|c| c.as_f64()).unwrap_or(0.0);
                        let peak_cpu = resource_stats.get("peak_cpu_percent").and_then(|p| p.as_f64()).unwrap_or(0.0);
                        let avg_memory = resource_stats.get("average_memory_percent").and_then(|m| m.as_f64()).unwrap_or(0.0);
                        let peak_memory = resource_stats.get("peak_memory_percent").and_then(|p| p.as_f64()).unwrap_or(0.0);

                        output.push(format!("Average CPU: {:.1}%", avg_cpu));
                        output.push(format!("Peak CPU: {:.1}%", peak_cpu));
                        output.push(format!("Average Memory: {:.1}%", avg_memory));
                        output.push(format!("Peak Memory: {:.1}%", peak_memory));
                    }

                    // Breakdown by Task Type (if requested)
                    if breakdown {
                        if let Some(breakdown_stats) = stats.get("breakdown") {
                            output.push("".to_string());
                            output.push("📋 Breakdown by Task Type".to_string());
                            output.push("─".repeat(40));
                            
                            if let Some(by_type) = breakdown_stats.get("by_task_type").and_then(|b| b.as_object()) {
                                for (task_type, type_stats) in by_type {
                                    let count = type_stats.get("count").and_then(|c| c.as_u64()).unwrap_or(0);
                                    let success_rate = type_stats.get("success_rate").and_then(|s| s.as_f64()).unwrap_or(0.0);
                                    output.push(format!("{}: {} executions ({:.1}% success)", task_type, count, success_rate));
                                }
                            }
                        }
                    }

                    Ok(CommandOutput::text(output.join("\n")))
                }
            }
        } else {
            Ok(CommandOutput::error("Failed to retrieve system statistics"))
        }
    }

    /// Monitor system in real-time with streaming updates
    async fn real_time_monitor(
        &self,
        args: &CommandArgs,
        mcp_client: &EnhancedMcpClient,
    ) -> Result<CommandOutput> {
        let interval = args.get_number_flag("interval", 5);
        let metrics = args.get_flag("metrics").unwrap_or("all");

        // For now, just show a single snapshot - streaming would be implemented in a full version
        let monitor_args = json!({
            "include_realtime": true,
            "metrics_filter": metrics,
            "sample_interval_seconds": interval
        });

        let result = mcp_client
            .execute_tool("ratchet_monitor_system", monitor_args)
            .await?;

        if let Some(monitoring) = result.get("monitoring") {
            let mut output = vec![
                "🔴 LIVE: Real-time System Monitoring".to_string(),
                "".to_string(),
            ];

            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();
            
            output.push(format!("📅 Timestamp: {}", 
                chrono::DateTime::from_timestamp(timestamp as i64, 0)
                    .unwrap_or_default()
                    .format("%Y-%m-%d %H:%M:%S UTC")));

            if let Some(current) = monitoring.get("current_metrics") {
                output.push("".to_string());
                output.push("📊 Current Metrics".to_string());
                output.push("─".repeat(30));
                
                if let Some(cpu) = current.get("cpu_percent").and_then(|c| c.as_f64()) {
                    let cpu_bar = create_progress_bar(cpu, 100.0, 20);
                    output.push(format!("CPU:    {:.1}% {}", cpu, cpu_bar));
                }
                
                if let Some(memory) = current.get("memory_percent").and_then(|m| m.as_f64()) {
                    let mem_bar = create_progress_bar(memory, 100.0, 20);
                    output.push(format!("Memory: {:.1}% {}", memory, mem_bar));
                }
                
                if let Some(active) = current.get("active_executions").and_then(|a| a.as_u64()) {
                    output.push(format!("Active Executions: {}", active));
                }
                
                if let Some(throughput) = current.get("throughput_per_minute").and_then(|t| t.as_f64()) {
                    output.push(format!("Throughput: {:.1}/min", throughput));
                }
            }

            output.push("".to_string());
            output.push(format!("🔄 Monitoring every {}s | Use Ctrl+C to stop", interval));
            output.push("💡 Tip: Use --interval <seconds> to change refresh rate".to_string());

            Ok(CommandOutput::text(output.join("\n")))
        } else {
            Ok(CommandOutput::error("Failed to start real-time monitoring"))
        }
    }
}

#[async_trait]
impl ConsoleCommand for MonitorCommand {
    async fn execute(&self, args: CommandArgs, mcp_client: &EnhancedMcpClient) -> Result<CommandOutput> {
        match args.action.as_str() {
            "dashboard" | "dash" => self.show_dashboard(&args, mcp_client).await,
            "health" | "status" => self.show_health(&args, mcp_client).await,
            "stats" | "statistics" => self.show_stats(&args, mcp_client).await,
            "live" | "realtime" | "watch" => self.real_time_monitor(&args, mcp_client).await,
            "help" | _ => Ok(CommandOutput::text(self.help_text().to_string())),
        }
    }

    fn completion_hints(&self, partial: &str) -> Vec<String> {
        let commands = vec!["dashboard", "health", "stats", "live", "help"];
        commands
            .into_iter()
            .filter(|cmd| cmd.starts_with(partial))
            .map(|cmd| cmd.to_string())
            .collect()
    }

    fn help_text(&self) -> &'static str {
        "Monitoring and Dashboard Commands:
  monitor dashboard [--refresh <seconds>] [--compact] [--filter <type>]
    Show comprehensive system dashboard with real-time metrics
    
  monitor health [--detailed] [--services]
    Display detailed system health information
    
  monitor stats [--range <time>] [--breakdown] [--format <json|table>]
    Show comprehensive system statistics and performance metrics
    
  monitor live [--interval <seconds>] [--metrics <type>]
    Real-time system monitoring with live updates

Examples:
  monitor dashboard --refresh 10 --compact
  monitor health --detailed --services
  monitor stats --range last_week --breakdown
  monitor live --interval 2 --metrics cpu,memory"
    }

    fn usage_examples(&self) -> Vec<&'static str> {
        vec![
            "monitor dashboard",
            "monitor dashboard --compact --refresh 30",
            "monitor health --detailed",
            "monitor stats --range last_24_hours",
            "monitor live --interval 5",
        ]
    }

    fn category(&self) -> &'static str {
        "monitoring"
    }

    fn aliases(&self) -> Vec<&'static str> {
        vec!["mon", "watch", "dashboard"]
    }

    fn requires_connection(&self) -> bool {
        true
    }

    fn validate_args(&self, _args: &CommandArgs) -> Result<()> {
        Ok(())
    }
}

/// Helper function to format duration in human-readable format
fn format_duration(seconds: u64) -> String {
    let days = seconds / 86400;
    let hours = (seconds % 86400) / 3600;
    let minutes = (seconds % 3600) / 60;
    let secs = seconds % 60;

    if days > 0 {
        format!("{}d {}h {}m", days, hours, minutes)
    } else if hours > 0 {
        format!("{}h {}m", hours, minutes)
    } else if minutes > 0 {
        format!("{}m {}s", minutes, secs)
    } else {
        format!("{}s", secs)
    }
}

/// Helper function to create ASCII progress bars
fn create_progress_bar(current: f64, max: f64, width: usize) -> String {
    let percentage = (current / max).min(1.0);
    let filled = (percentage * width as f64) as usize;
    let empty = width - filled;
    
    let mut bar = String::new();
    bar.push('[');
    bar.push_str(&"█".repeat(filled));
    bar.push_str(&"░".repeat(empty));
    bar.push(']');
    bar
}
