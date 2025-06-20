//! Service registry and dependency injection

use async_trait::async_trait;
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::Arc;

use crate::error::{Result, ServiceError};

/// Service provider trait for dependency injection
#[async_trait]
pub trait ServiceProvider: Send + Sync {
    /// Get a service by type
    async fn get<T: Send + Sync + 'static>(&self) -> Result<Arc<T>>;

    /// Check if a service is registered
    fn has<T: Send + Sync + 'static>(&self) -> bool;
}

/// Service registry for managing dependencies
pub struct ServiceRegistry {
    /// Singleton services
    singletons: HashMap<TypeId, Arc<dyn Any + Send + Sync>>,

    /// Service factories
    factories: HashMap<TypeId, Box<dyn ServiceFactory>>,

    /// Service aliases for interface-to-implementation mapping
    aliases: HashMap<TypeId, TypeId>,
}

impl ServiceRegistry {
    /// Create a new service registry
    pub fn new() -> Self {
        Self {
            singletons: HashMap::new(),
            factories: HashMap::new(),
            aliases: HashMap::new(),
        }
    }

    /// Register a singleton service
    pub fn register_singleton<T>(&mut self, service: T) -> &mut Self
    where
        T: Send + Sync + 'static,
    {
        self.singletons
            .insert(TypeId::of::<T>(), Arc::new(service) as Arc<dyn Any + Send + Sync>);
        self
    }

    /// Register a service factory
    pub fn register_factory<T, F>(&mut self, factory: F) -> &mut Self
    where
        T: Send + Sync + 'static,
        F: Fn() -> Result<T> + Send + Sync + 'static,
    {
        self.factories
            .insert(TypeId::of::<T>(), Box::new(TypedServiceFactory::<T, F>::new(factory)));
        self
    }

    /// Register an async service factory
    pub fn register_async_factory<T, F, Fut>(&mut self, factory: F) -> &mut Self
    where
        T: Send + Sync + 'static,
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<T>> + Send + Sync + 'static,
    {
        self.factories.insert(
            TypeId::of::<T>(),
            Box::new(AsyncServiceFactory::<T, F, Fut>::new(factory)),
        );
        self
    }

    /// Register a service alias (interface to implementation)
    pub fn register_alias<I, T>(&mut self) -> &mut Self
    where
        I: ?Sized + 'static,
        T: 'static,
    {
        self.aliases.insert(TypeId::of::<I>(), TypeId::of::<T>());
        self
    }

    /// Resolve a service
    pub async fn resolve<T: Send + Sync + 'static>(&self) -> Result<Arc<T>> {
        let type_id = self.resolve_type_id::<T>();

        // Check singletons first
        if let Some(service) = self.singletons.get(&type_id) {
            let any_arc = service.clone();
            return any_arc
                .downcast::<T>()
                .map_err(|_| ServiceError::NotFound(std::any::type_name::<T>().to_string()).into());
        }

        // Try factory
        if let Some(factory) = self.factories.get(&type_id) {
            let service = factory.create().await?;
            return service
                .downcast::<T>()
                .map(|boxed| Arc::from(*boxed))
                .map_err(|_| ServiceError::NotFound(std::any::type_name::<T>().to_string()).into());
        }

        Err(ServiceError::NotFound(std::any::type_name::<T>().to_string()).into())
    }

    /// Check if a service is registered
    pub fn has<T: Send + Sync + 'static>(&self) -> bool {
        let type_id = self.resolve_type_id::<T>();
        self.singletons.contains_key(&type_id) || self.factories.contains_key(&type_id)
    }

    /// Resolve type ID considering aliases
    fn resolve_type_id<T: 'static>(&self) -> TypeId {
        let original = TypeId::of::<T>();
        self.aliases.get(&original).copied().unwrap_or(original)
    }

    /// Create a scoped registry (for request-scoped services)
    pub fn create_scope(&self) -> ServiceScope {
        ServiceScope::new(self)
    }
}

impl Default for ServiceRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Service factory trait
#[async_trait]
trait ServiceFactory: Send + Sync {
    async fn create(&self) -> Result<Box<dyn Any + Send + Sync>>;
}

/// Typed service factory for sync creation
struct TypedServiceFactory<T, F> {
    factory: F,
    _phantom: std::marker::PhantomData<T>,
}

impl<T, F> TypedServiceFactory<T, F>
where
    F: Fn() -> Result<T>,
{
    fn new(factory: F) -> Self {
        Self {
            factory,
            _phantom: std::marker::PhantomData,
        }
    }
}

#[async_trait]
impl<T, F> ServiceFactory for TypedServiceFactory<T, F>
where
    T: Send + Sync + 'static,
    F: Fn() -> Result<T> + Send + Sync,
{
    async fn create(&self) -> Result<Box<dyn Any + Send + Sync>> {
        let service = (self.factory)()?;
        Ok(Box::new(service) as Box<dyn Any + Send + Sync>)
    }
}

/// Async service factory
struct AsyncServiceFactory<T, F, Fut> {
    factory: F,
    _phantom: std::marker::PhantomData<(T, Fut)>,
}

impl<T, F, Fut> AsyncServiceFactory<T, F, Fut>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<T>>,
{
    fn new(factory: F) -> Self {
        Self {
            factory,
            _phantom: std::marker::PhantomData,
        }
    }
}

#[async_trait]
impl<T, F, Fut> ServiceFactory for AsyncServiceFactory<T, F, Fut>
where
    T: Send + Sync + 'static,
    F: Fn() -> Fut + Send + Sync,
    Fut: std::future::Future<Output = Result<T>> + Send + Sync + 'static,
{
    async fn create(&self) -> Result<Box<dyn Any + Send + Sync>> {
        let service = (self.factory)().await?;
        Ok(Box::new(service) as Box<dyn Any + Send + Sync>)
    }
}

/// Scoped service registry for request-scoped services
pub struct ServiceScope<'a> {
    parent: &'a ServiceRegistry,
    scoped_services: HashMap<TypeId, Arc<dyn Any + Send + Sync>>,
}

impl<'a> ServiceScope<'a> {
    /// Create a new service scope
    fn new(parent: &'a ServiceRegistry) -> Self {
        Self {
            parent,
            scoped_services: HashMap::new(),
        }
    }

    /// Register a scoped service
    pub fn register_scoped<T>(&mut self, service: T) -> &mut Self
    where
        T: Send + Sync + 'static,
    {
        self.scoped_services
            .insert(TypeId::of::<T>(), Arc::new(service) as Arc<dyn Any + Send + Sync>);
        self
    }

    /// Resolve a service (checks scope first, then parent)
    pub async fn resolve<T: Send + Sync + 'static>(&self) -> Result<Arc<T>> {
        let type_id = TypeId::of::<T>();

        // Check scoped services first
        if let Some(service) = self.scoped_services.get(&type_id) {
            let any_arc = service.clone();
            return any_arc
                .downcast::<T>()
                .map_err(|_| ServiceError::NotFound(std::any::type_name::<T>().to_string()).into());
        }

        // Fall back to parent registry
        self.parent.resolve::<T>().await
    }
}

/// Default service provider implementation
pub struct DefaultServiceProvider {
    registry: Arc<ServiceRegistry>,
}

impl DefaultServiceProvider {
    /// Create a new service provider
    pub fn new(registry: ServiceRegistry) -> Self {
        Self {
            registry: Arc::new(registry),
        }
    }

    /// Get the underlying registry
    pub fn registry(&self) -> &ServiceRegistry {
        &self.registry
    }
}

#[async_trait]
impl ServiceProvider for DefaultServiceProvider {
    async fn get<T: Send + Sync + 'static>(&self) -> Result<Arc<T>> {
        self.registry.resolve::<T>().await
    }

    fn has<T: Send + Sync + 'static>(&self) -> bool {
        self.registry.has::<T>()
    }
}

/// Builder for configuring services
pub struct ServiceBuilder {
    registry: ServiceRegistry,
}

impl ServiceBuilder {
    /// Create a new service builder
    pub fn new() -> Self {
        Self {
            registry: ServiceRegistry::new(),
        }
    }

    /// Register a singleton service
    pub fn singleton<T>(mut self, service: T) -> Self
    where
        T: Send + Sync + 'static,
    {
        self.registry.register_singleton(service);
        self
    }

    /// Register a factory
    pub fn factory<T, F>(mut self, factory: F) -> Self
    where
        T: Send + Sync + 'static,
        F: Fn() -> Result<T> + Send + Sync + 'static,
    {
        self.registry.register_factory::<T, F>(factory);
        self
    }

    /// Register an async factory
    pub fn async_factory<T, F, Fut>(mut self, factory: F) -> Self
    where
        T: Send + Sync + 'static,
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<T>> + Send + Sync + 'static,
    {
        self.registry.register_async_factory::<T, F, Fut>(factory);
        self
    }

    /// Register an alias
    pub fn alias<I, T>(mut self) -> Self
    where
        I: ?Sized + 'static,
        T: 'static,
    {
        self.registry.register_alias::<I, T>();
        self
    }

    /// Build the service provider
    pub fn build(self) -> DefaultServiceProvider {
        DefaultServiceProvider::new(self.registry)
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

    #[derive(Debug, Clone)]
    struct TestService {
        value: String,
    }

    trait TestTrait: Send + Sync {
        fn get_value(&self) -> &str;
    }

    impl TestTrait for TestService {
        fn get_value(&self) -> &str {
            &self.value
        }
    }

    #[tokio::test]
    async fn test_singleton_registration() {
        let mut registry = ServiceRegistry::new();
        let service = TestService {
            value: "test".to_string(),
        };

        registry.register_singleton(service.clone());

        let resolved = registry.resolve::<TestService>().await.unwrap();
        assert_eq!(resolved.value, "test");
    }

    #[tokio::test]
    async fn test_factory_registration() {
        let mut registry = ServiceRegistry::new();

        registry.register_factory(|| {
            Ok(TestService {
                value: "factory".to_string(),
            })
        });

        let resolved = registry.resolve::<TestService>().await.unwrap();
        assert_eq!(resolved.value, "factory");
    }

    #[tokio::test]
    async fn test_async_factory_registration() {
        let mut registry = ServiceRegistry::new();

        registry.register_async_factory(|| async {
            Ok(TestService {
                value: "async_factory".to_string(),
            })
        });

        let resolved = registry.resolve::<TestService>().await.unwrap();
        assert_eq!(resolved.value, "async_factory");
    }

    #[tokio::test]
    async fn test_scoped_services() {
        let mut registry = ServiceRegistry::new();
        registry.register_singleton(TestService {
            value: "parent".to_string(),
        });

        let mut scope = registry.create_scope();
        scope.register_scoped(TestService {
            value: "scoped".to_string(),
        });

        let resolved = scope.resolve::<TestService>().await.unwrap();
        assert_eq!(resolved.value, "scoped");
    }

    #[tokio::test]
    async fn test_service_builder() {
        let provider = ServiceBuilder::new()
            .singleton(TestService {
                value: "builder".to_string(),
            })
            .build();

        let service = provider.get::<TestService>().await.unwrap();
        assert_eq!(service.value, "builder");
        assert!(provider.has::<TestService>());
    }

    #[tokio::test]
    async fn test_missing_service() {
        let registry = ServiceRegistry::new();
        let result = registry.resolve::<TestService>().await;
        assert!(result.is_err());
    }
}
