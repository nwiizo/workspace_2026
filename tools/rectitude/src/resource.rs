//! Resource lifecycle management for automatic cleanup
//!
//! Provides a `Resource` trait and `ResourceManager` for automatic cleanup
//! of resources (connections, files, etc.) after scenario execution.
//!
//! # Example
//! ```ignore
//! use rectitude::resource::{Resource, ResourceManager};
//!
//! struct TempFile {
//!     path: String,
//! }
//!
//! #[async_trait::async_trait]
//! impl Resource for TempFile {
//!     async fn dispose(&self) -> anyhow::Result<()> {
//!         std::fs::remove_file(&self.path)?;
//!         Ok(())
//!     }
//!
//!     fn name(&self) -> &str {
//!         &self.path
//!     }
//! }
//! ```

use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, warn};

/// Result type for resource operations
pub type ResourceResult<T> = anyhow::Result<T>;

/// Trait for resources that can be automatically disposed
///
/// Resources are disposed in reverse order of registration (LIFO).
#[async_trait]
pub trait Resource: Send + Sync {
    /// Dispose of the resource
    ///
    /// This is called automatically when the scenario completes.
    /// Should handle cleanup gracefully and log any errors.
    async fn dispose(&self) -> ResourceResult<()>;

    /// Optional name for logging purposes
    fn name(&self) -> &str {
        "unnamed"
    }
}

/// Manager for tracking and disposing resources
#[derive(Default)]
pub struct ResourceManager {
    resources: RwLock<Vec<Arc<dyn Resource>>>,
}

impl ResourceManager {
    /// Create a new empty resource manager
    pub fn new() -> Self {
        Self {
            resources: RwLock::new(Vec::new()),
        }
    }

    /// Register a resource for automatic cleanup
    ///
    /// Resources are disposed in reverse order of registration.
    pub async fn register(&self, resource: Arc<dyn Resource>) {
        let mut resources = self.resources.write().await;
        debug!("Registered resource: {}", resource.name());
        resources.push(resource);
    }

    /// Register a resource and return it for use
    ///
    /// Convenience method that registers and returns the resource.
    pub async fn register_and_use<R: Resource + 'static>(&self, resource: R) -> Arc<R> {
        let arc = Arc::new(resource);
        self.register(arc.clone() as Arc<dyn Resource>).await;
        arc
    }

    /// Get the number of registered resources
    pub async fn count(&self) -> usize {
        self.resources.read().await.len()
    }

    /// Dispose all resources in reverse order (LIFO)
    ///
    /// Errors are logged but don't stop the disposal of other resources.
    pub async fn dispose_all(&self) {
        let resources = {
            let mut guard = self.resources.write().await;
            std::mem::take(&mut *guard)
        };

        // Dispose in reverse order
        for resource in resources.into_iter().rev() {
            let name = resource.name().to_string();
            debug!("Disposing resource: {}", name);

            if let Err(e) = resource.dispose().await {
                warn!("Failed to dispose resource '{}': {}", name, e);
            }
        }
    }

    /// Clear all resources without disposing
    ///
    /// Use with caution - this will leak resources.
    pub async fn clear(&self) {
        let mut resources = self.resources.write().await;
        resources.clear();
    }
}

/// Guard that disposes a resource when dropped
pub struct ResourceGuard<R: Resource + 'static> {
    resource: Option<Arc<R>>,
}

impl<R: Resource> ResourceGuard<R> {
    /// Create a new guard that will dispose the resource when dropped
    pub fn new(resource: Arc<R>) -> Self {
        Self {
            resource: Some(resource),
        }
    }

    /// Get a reference to the resource
    pub fn get(&self) -> Option<&Arc<R>> {
        self.resource.as_ref()
    }

    /// Take the resource without disposing (transfers ownership)
    pub fn take(mut self) -> Option<Arc<R>> {
        self.resource.take()
    }
}

impl<R: Resource + 'static> Drop for ResourceGuard<R> {
    fn drop(&mut self) {
        if let Some(resource) = self.resource.take() {
            // Spawn a task to dispose the resource
            tokio::spawn(async move {
                if let Err(e) = resource.dispose().await {
                    warn!("Failed to dispose resource '{}': {}", resource.name(), e);
                }
            });
        }
    }
}

/// Simple cleanup function wrapper as a Resource
pub struct CleanupResource<F>
where
    F: Fn() -> std::pin::Pin<Box<dyn std::future::Future<Output = ResourceResult<()>> + Send>>
        + Send
        + Sync,
{
    name: String,
    cleanup: F,
}

impl<F> CleanupResource<F>
where
    F: Fn() -> std::pin::Pin<Box<dyn std::future::Future<Output = ResourceResult<()>> + Send>>
        + Send
        + Sync,
{
    /// Create a new cleanup resource with a name and cleanup function
    pub fn new(name: impl Into<String>, cleanup: F) -> Self {
        Self {
            name: name.into(),
            cleanup,
        }
    }
}

#[async_trait]
impl<F> Resource for CleanupResource<F>
where
    F: Fn() -> std::pin::Pin<Box<dyn std::future::Future<Output = ResourceResult<()>> + Send>>
        + Send
        + Sync,
{
    async fn dispose(&self) -> ResourceResult<()> {
        (self.cleanup)().await
    }

    fn name(&self) -> &str {
        &self.name
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

    struct TestResource {
        name: String,
        disposed: Arc<AtomicBool>,
    }

    #[async_trait]
    impl Resource for TestResource {
        async fn dispose(&self) -> ResourceResult<()> {
            self.disposed.store(true, Ordering::SeqCst);
            Ok(())
        }

        fn name(&self) -> &str {
            &self.name
        }
    }

    #[tokio::test]
    async fn test_resource_manager_basic() {
        let manager = ResourceManager::new();
        let disposed = Arc::new(AtomicBool::new(false));

        let resource = Arc::new(TestResource {
            name: "test".to_string(),
            disposed: disposed.clone(),
        });

        manager.register(resource as Arc<dyn Resource>).await;
        assert_eq!(manager.count().await, 1);

        manager.dispose_all().await;
        assert!(disposed.load(Ordering::SeqCst));
        assert_eq!(manager.count().await, 0);
    }

    #[tokio::test]
    async fn test_lifo_disposal_order() {
        let manager = ResourceManager::new();
        let order = Arc::new(AtomicUsize::new(0));

        struct OrderedResource {
            name: String,
            expected_order: usize,
            order_tracker: Arc<AtomicUsize>,
        }

        #[async_trait]
        impl Resource for OrderedResource {
            async fn dispose(&self) -> ResourceResult<()> {
                let actual = self.order_tracker.fetch_add(1, Ordering::SeqCst);
                assert_eq!(
                    actual, self.expected_order,
                    "Resource {} disposed in wrong order",
                    self.name
                );
                Ok(())
            }

            fn name(&self) -> &str {
                &self.name
            }
        }

        // Register in order: first, second, third
        // Should dispose in order: third (0), second (1), first (2)
        manager
            .register(Arc::new(OrderedResource {
                name: "first".to_string(),
                expected_order: 2, // Should be disposed last
                order_tracker: order.clone(),
            }) as Arc<dyn Resource>)
            .await;

        manager
            .register(Arc::new(OrderedResource {
                name: "second".to_string(),
                expected_order: 1,
                order_tracker: order.clone(),
            }) as Arc<dyn Resource>)
            .await;

        manager
            .register(Arc::new(OrderedResource {
                name: "third".to_string(),
                expected_order: 0, // Should be disposed first
                order_tracker: order.clone(),
            }) as Arc<dyn Resource>)
            .await;

        manager.dispose_all().await;
        assert_eq!(order.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn test_register_and_use() {
        let manager = ResourceManager::new();
        let disposed = Arc::new(AtomicBool::new(false));

        let resource = TestResource {
            name: "test".to_string(),
            disposed: disposed.clone(),
        };

        let arc = manager.register_and_use(resource).await;
        assert_eq!(arc.name, "test");
        assert_eq!(manager.count().await, 1);

        manager.dispose_all().await;
        assert!(disposed.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn test_cleanup_resource() {
        let called = Arc::new(AtomicBool::new(false));
        let called_clone = called.clone();

        let resource = CleanupResource::new("cleanup-test", move || {
            let called = called_clone.clone();
            Box::pin(async move {
                called.store(true, Ordering::SeqCst);
                Ok(())
            })
        });

        resource.dispose().await.unwrap();
        assert!(called.load(Ordering::SeqCst));
    }
}
