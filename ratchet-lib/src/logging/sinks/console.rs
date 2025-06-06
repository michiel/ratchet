use crate::logging::{logger::LogSink, LogEvent, LogLevel};
use colored::Colorize;
use std::io::Write;

pub struct ConsoleSink {
    min_level: LogLevel,
    use_json: bool,
}

impl ConsoleSink {
    pub fn new(min_level: LogLevel) -> Self {
        Self {
            min_level,
            use_json: false,
        }
    }

    pub fn json_format(mut self) -> Self {
        self.use_json = true;
        self
    }

    fn format_event(&self, event: &LogEvent) -> String {
        if self.use_json {
            // JSON format for production
            serde_json::to_string(event).unwrap_or_else(|_| event.message.clone())
        } else {
            // Human-readable format for development
            let level_str = match event.level {
                LogLevel::Trace => "TRACE".dimmed(),
                LogLevel::Debug => "DEBUG".blue(),
                LogLevel::Info => "INFO ".green(),
                LogLevel::Warn => "WARN ".yellow(),
                LogLevel::Error => "ERROR".red(),
            };

            let timestamp = event.timestamp.format("%Y-%m-%d %H:%M:%S%.3f");
            let mut output = format!("{} {} {}", timestamp, level_str, event.message);

            // Add error details if present
            if let Some(error) = &event.error {
                output.push_str(&format!(
                    "\n  Error: {} ({})",
                    error.message, error.error_code
                ));
                output.push_str(&format!("\n  Type: {}", error.error_type));
                output.push_str(&format!("\n  Retryable: {}", error.is_retryable));

                if !error.suggestions.immediate.is_empty() {
                    output.push_str("\n  Suggestions:");
                    for suggestion in &error.suggestions.immediate {
                        output.push_str(&format!("\n    - {}", suggestion));
                    }
                }
            }

            // Add important fields
            if !event.fields.is_empty() {
                let important_fields: Vec<(&str, &serde_json::Value)> = event
                    .fields
                    .iter()
                    .filter(|(k, _)| {
                        matches!(
                            k.as_str(),
                            "task_id" | "job_id" | "execution_id" | "error_count"
                        )
                    })
                    .map(|(k, v)| (k.as_str(), v))
                    .collect();

                if !important_fields.is_empty() {
                    output.push_str(" [");
                    let field_strs: Vec<String> = important_fields
                        .iter()
                        .map(|(k, v)| format!("{}={}", k, v))
                        .collect();
                    output.push_str(&field_strs.join(" "));
                    output.push(']');
                }
            }

            // Add trace context
            if let (Some(trace_id), Some(span_id)) = (&event.trace_id, &event.span_id) {
                output.push_str(
                    &format!(
                        " trace={} span={}",
                        &trace_id[..8], // Show first 8 chars
                        &span_id[..8]
                    )
                    .dimmed()
                    .to_string(),
                );
            }

            output
        }
    }
}

impl LogSink for ConsoleSink {
    fn log(&self, event: LogEvent) {
        if event.level < self.min_level {
            return;
        }

        let formatted = self.format_event(&event);

        // Write to stderr for errors, stdout for others
        if event.level >= LogLevel::Error {
            eprintln!("{}", formatted);
        } else {
            println!("{}", formatted);
        }
    }

    fn flush(&self) {
        let _ = std::io::stdout().flush();
        let _ = std::io::stderr().flush();
    }
}
