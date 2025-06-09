// Re-export core service interfaces from ratchet-interfaces
pub use ratchet_interfaces::{Service, ServiceHealth, ServiceMetrics, HealthStatus};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

// Note: Service, ServiceHealth, ServiceMetrics, and HealthStatus are now re-exported from ratchet-interfaces

/// Service registry for managing multiple services
pub struct ServiceRegistry {
    services: HashMap<String, Arc<dyn ServiceInfo>>,
}

impl ServiceRegistry {
    pub fn new() -> Self {
        Self {
            services: HashMap::new(),
        }
    }

    pub fn register<S>(&mut self, service: Arc<S>) -> Result<(), RegistryError>
    where
        S: Service + 'static,
    {
        let name = service.name().to_string();
        if self.services.contains_key(&name) {
            return Err(RegistryError::ServiceAlreadyRegistered(name));
        }

        let service_info = Arc::new(ServiceInfoImpl::new(service));
        self.services.insert(name, service_info);
        Ok(())
    }

    pub fn get<S>(&self, name: &str) -> Option<Arc<S>>
    where
        S: Service + 'static,
    {
        self.services
            .get(name)?
            .as_any()
            .downcast_ref::<ServiceInfoImpl<S>>()
            .map(|info| info.service.clone())
    }

    pub async fn health_check_all(&self) -> HashMap<String, ServiceHealth> {
        let mut results = HashMap::new();

        for (name, service) in &self.services {
            match service.health_check().await {
                Ok(health) => {
                    results.insert(name.clone(), health);
                }
                Err(_) => {
                    results.insert(
                        name.clone(),
                        ServiceHealth::unhealthy("Health check failed"),
                    );
                }
            }
        }

        results
    }

    pub async fn shutdown_all(
        &self,
    ) -> Result<(), Vec<(String, Box<dyn std::error::Error + Send + Sync>)>> {
        let mut errors = Vec::new();

        for (name, service) in &self.services {
            if let Err(e) = service.shutdown().await {
                errors.push((name.clone(), e));
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    pub fn list_services(&self) -> Vec<String> {
        self.services.keys().cloned().collect()
    }

    pub fn get_metrics(&self, name: &str) -> Option<ServiceMetrics> {
        self.services.get(name)?.get_metrics()
    }
}

impl Default for ServiceRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Service information trait for type erasure
#[async_trait]
trait ServiceInfo: Send + Sync {
    fn as_any(&self) -> &dyn std::any::Any;
    async fn health_check(&self)
        -> Result<ServiceHealth, Box<dyn std::error::Error + Send + Sync>>;
    async fn shutdown(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    fn get_metrics(&self) -> Option<ServiceMetrics>;
}

/// Implementation of ServiceInfo for concrete services
struct ServiceInfoImpl<S> {
    service: Arc<S>,
}

impl<S> ServiceInfoImpl<S> {
    fn new(service: Arc<S>) -> Self {
        Self { service }
    }
}

#[async_trait]
impl<S> ServiceInfo for ServiceInfoImpl<S>
where
    S: Service + 'static,
{
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    async fn health_check(
        &self,
    ) -> Result<ServiceHealth, Box<dyn std::error::Error + Send + Sync>> {
        self.service
            .health_check()
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
    }

    async fn shutdown(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.service
            .shutdown()
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
    }

    fn get_metrics(&self) -> Option<ServiceMetrics> {
        Some(self.service.metrics())
    }
}

/// Registry errors
#[derive(Debug, thiserror::Error)]
pub enum RegistryError {
    #[error("Service '{0}' is already registered")]
    ServiceAlreadyRegistered(String),

    #[error("Service '{0}' not found")]
    ServiceNotFound(String),

    #[error("Service initialization failed: {0}")]
    InitializationFailed(Box<dyn std::error::Error + Send + Sync>),
}

/// Convenience macros for service implementation
#[macro_export]
macro_rules! impl_service_base {
    ($service:ty, $error:ty, $config:ty, $name:expr) => {
        #[async_trait::async_trait]
        impl $crate::services::base::Service for $service {
            type Error = $error;
            type Config = $config;

            async fn initialize(config: Self::Config) -> Result<Self, Self::Error> {
                Self::new(config).await
            }

            fn name(&self) -> &'static str {
                $name
            }

            async fn health_check(
                &self,
            ) -> Result<$crate::services::base::ServiceHealth, Self::Error> {
                // Default implementation - override if needed
                Ok($crate::services::base::ServiceHealth::healthy())
            }

            async fn shutdown(&self) -> Result<(), Self::Error> {
                // Default implementation - override if needed
                Ok(())
            }
        }
    };
}

/// Service builder for easier service composition
pub struct ServiceBuilder {
    registry: ServiceRegistry,
}

impl ServiceBuilder {
    pub fn new() -> Self {
        Self {
            registry: ServiceRegistry::new(),
        }
    }

    pub async fn add_service<S, F>(mut self, factory: F) -> Result<Self, RegistryError>
    where
        S: Service + 'static,
        F: FnOnce() -> Result<Arc<S>, S::Error>,
    {
        let service = factory().map_err(|e| RegistryError::InitializationFailed(Box::new(e)))?;

        self.registry.register(service)?;
        Ok(self)
    }

    pub fn build(self) -> ServiceRegistry {
        self.registry
    }
}

impl Default for ServiceBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    // Mock service for testing
    struct MockService {
        name: &'static str,
        call_count: AtomicU64,
    }

    impl MockService {
        fn new(name: &'static str) -> Self {
            Self {
                name,
                call_count: AtomicU64::new(0),
            }
        }
    }

    #[derive(Debug, thiserror::Error)]
    #[error("Mock service error")]
    struct MockError;

    #[async_trait]
    impl Service for MockService {
        type Error = MockError;
        type Config = ();

        async fn initialize(_config: Self::Config) -> Result<Self, Self::Error> {
            Ok(Self::new("mock"))
        }

        fn name(&self) -> &'static str {
            self.name
        }

        async fn health_check(&self) -> Result<ServiceHealth, Self::Error> {
            self.call_count.fetch_add(1, Ordering::Relaxed);
            Ok(ServiceHealth::healthy().with_message("Mock service is healthy"))
        }

        async fn shutdown(&self) -> Result<(), Self::Error> {
            Ok(())
        }

        fn metrics(&self) -> ServiceMetrics {
            ServiceMetrics {
                requests_total: self.call_count.load(Ordering::Relaxed),
                requests_failed: 0,
                average_latency_ms: 10.0,
                uptime_seconds: 3600,
                memory_usage_bytes: Some(1024 * 1024),
                custom_metrics: HashMap::new(),
            }
        }
    }

    #[tokio::test]
    async fn test_service_registry() {
        let mut registry = ServiceRegistry::new();
        let service = Arc::new(MockService::new("test-service"));

        // Register service
        registry.register(service.clone()).unwrap();

        // Get service back
        let retrieved: Option<Arc<MockService>> = registry.get("test-service");
        assert!(retrieved.is_some());

        // Health check
        let health_results = registry.health_check_all().await;
        assert_eq!(health_results.len(), 1);
        assert_eq!(health_results["test-service"].status, HealthStatus::Healthy);

        // Metrics
        let metrics = registry.get_metrics("test-service").unwrap();
        assert_eq!(metrics.requests_total, 1); // From health check
    }

    #[tokio::test]
    async fn test_service_builder() {
        let registry = ServiceBuilder::new()
            .add_service(|| Ok(Arc::new(MockService::new("service1"))))
            .await
            .unwrap()
            .add_service(|| Ok(Arc::new(MockService::new("service2"))))
            .await
            .unwrap()
            .build();

        let services = registry.list_services();
        assert_eq!(services.len(), 2);
        assert!(services.contains(&"service1".to_string()));
        assert!(services.contains(&"service2".to_string()));
    }

    #[test]
    fn test_service_health() {
        let health = ServiceHealth::healthy()
            .with_message("All systems operational")
            .with_latency(50)
            .with_metadata("version", "1.0.0");

        assert_eq!(health.status, HealthStatus::Healthy);
        assert_eq!(health.message, Some("All systems operational".to_string()));
        assert_eq!(health.latency_ms, Some(50));
        assert_eq!(health.metadata["version"], "1.0.0");
    }

    #[test]
    fn test_service_metrics() {
        let metrics = ServiceMetrics {
            requests_total: 100,
            requests_failed: 5,
            average_latency_ms: 25.0,
            uptime_seconds: 3600,
            memory_usage_bytes: Some(1024 * 1024),
            custom_metrics: HashMap::new(),
        };

        assert!((metrics.success_rate() - 0.95).abs() < 0.0001);
        assert!((metrics.error_rate() - 0.05).abs() < 0.0001);
    }
}
