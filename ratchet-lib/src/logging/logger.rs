use super::{LogEvent, LogLevel, LogContext, Enricher, LogEnricher};
use async_trait::async_trait;
use std::sync::Arc;

#[async_trait(?Send)]
pub trait StructuredLogger: Send + Sync {
    /// Log an event
    fn log(&self, event: LogEvent);
    
    /// Create a new logger with additional context
    fn with_context(&self, context: LogContext) -> Box<dyn StructuredLogger>;
    
    /// Get the minimum log level
    fn min_level(&self) -> LogLevel;
}

/// Default logger implementation
pub struct DefaultLogger {
    min_level: LogLevel,
    enricher: Arc<LogEnricher>,
    sinks: Vec<Arc<dyn LogSink>>,
    context: Option<LogContext>,
}

impl DefaultLogger {
    pub fn new(min_level: LogLevel, sinks: Vec<Arc<dyn LogSink>>) -> Self {
        Self {
            min_level,
            enricher: Arc::new(LogEnricher::default()),
            sinks,
            context: None,
        }
    }

    pub fn with_enricher(mut self, enricher: Arc<LogEnricher>) -> Self {
        self.enricher = enricher;
        self
    }
}

#[async_trait(?Send)]
impl StructuredLogger for DefaultLogger {
    fn log(&self, mut event: LogEvent) {
        // Skip if below minimum level
        if !event.should_log(self.min_level) {
            return;
        }

        // Apply context if available
        if let Some(context) = &self.context {
            event.trace_id = Some(context.trace_id.clone());
            event.span_id = Some(context.span_id.clone());
            event.fields.extend(context.fields.clone());
        }

        // Enrich the event
        self.enricher.enrich(&mut event);

        // Send to all sinks
        for sink in &self.sinks {
            sink.log(event.clone());
        }
    }

    fn with_context(&self, context: LogContext) -> Box<dyn StructuredLogger> {
        Box::new(Self {
            min_level: self.min_level,
            enricher: self.enricher.clone(),
            sinks: self.sinks.clone(),
            context: Some(context),
        })
    }

    fn min_level(&self) -> LogLevel {
        self.min_level
    }
}

/// Trait for log output destinations
pub trait LogSink: Send + Sync {
    /// Write a log event
    fn log(&self, event: LogEvent);
    
    /// Flush any buffered events
    fn flush(&self);
}

/// Builder for creating loggers
pub struct LoggerBuilder {
    min_level: LogLevel,
    sinks: Vec<Arc<dyn LogSink>>,
    enrichers: Vec<Box<dyn Enricher>>,
}

impl LoggerBuilder {
    pub fn new() -> Self {
        Self {
            min_level: LogLevel::Info,
            sinks: Vec::new(),
            enrichers: Vec::new(),
        }
    }

    pub fn with_min_level(mut self, level: LogLevel) -> Self {
        self.min_level = level;
        self
    }

    pub fn add_sink(mut self, sink: Arc<dyn LogSink>) -> Self {
        self.sinks.push(sink);
        self
    }

    pub fn add_enricher(mut self, enricher: Box<dyn Enricher>) -> Self {
        self.enrichers.push(enricher);
        self
    }

    pub fn build(self) -> Arc<dyn StructuredLogger> {
        let enricher = Arc::new(LogEnricher::new(self.enrichers));
        let logger = DefaultLogger::new(self.min_level, self.sinks)
            .with_enricher(enricher);
        Arc::new(logger)
    }
}

impl Default for LoggerBuilder {
    fn default() -> Self {
        Self::new()
    }
}