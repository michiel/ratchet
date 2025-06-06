use serde::Serialize;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use uuid::Uuid;

tokio::task_local! {
    static CURRENT_CONTEXT: LogContext;
}

#[derive(Debug, Clone)]
pub struct LogContext {
    pub trace_id: String,
    pub span_id: String,
    pub fields: HashMap<String, JsonValue>,
}

impl LogContext {
    /// Create a new log context with generated IDs
    pub fn new() -> Self {
        Self {
            trace_id: Uuid::new_v4().to_string(),
            span_id: Self::generate_span_id(),
            fields: HashMap::new(),
        }
    }

    /// Create a child context with the same trace ID but new span ID
    pub fn child(&self) -> Self {
        Self {
            trace_id: self.trace_id.clone(),
            span_id: Self::generate_span_id(),
            fields: self.fields.clone(),
        }
    }

    /// Add a field to the context
    pub fn with_field(mut self, key: impl Into<String>, value: impl Serialize) -> Self {
        if let Ok(json_value) = serde_json::to_value(value) {
            self.fields.insert(key.into(), json_value);
        }
        self
    }

    /// Add multiple fields to the context
    pub fn with_fields(mut self, fields: HashMap<String, JsonValue>) -> Self {
        self.fields.extend(fields);
        self
    }

    /// Get the current context from task-local storage
    pub fn current() -> Self {
        CURRENT_CONTEXT
            .try_with(|ctx| ctx.clone())
            .unwrap_or_else(|_| Self::new())
    }

    /// Run a future with this context as current
    pub async fn scope<F, T>(self, f: F) -> T
    where
        F: std::future::Future<Output = T>,
    {
        CURRENT_CONTEXT.scope(self, f).await
    }

    /// Generate a short span ID
    fn generate_span_id() -> String {
        // Use first 8 bytes of UUID for shorter span IDs
        let uuid = Uuid::new_v4();
        format!("{:x}", uuid.as_u128() & 0xFFFFFFFFFFFFFFFF)
    }
}

impl Default for LogContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Extension trait for adding context to futures
pub trait WithLogContext: Sized {
    /// Attach a log context to this future
    fn with_context(self, context: LogContext) -> ContextScope<Self>;
}

impl<F> WithLogContext for F
where
    F: std::future::Future,
{
    fn with_context(self, context: LogContext) -> ContextScope<Self> {
        ContextScope {
            future: self,
            _context: context,
        }
    }
}

/// Future wrapper that provides log context
pub struct ContextScope<F> {
    future: F,
    _context: LogContext,
}

impl<F> std::future::Future for ContextScope<F>
where
    F: std::future::Future,
{
    type Output = F::Output;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };
        let future = unsafe { std::pin::Pin::new_unchecked(&mut this.future) };

        // This would need proper implementation with task-local storage
        // For now, just poll the inner future
        future.poll(cx)
    }
}
