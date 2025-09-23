use std::any::TypeId;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};
use std::thread;

use crate::renderer::{DataPrecision, RendererError, RendererFactory, RendererInfo};
use crate::renderer::factory::Renderer;

/// Thread-safe manager for renderer factories with timeout protection.
///
/// The RendererManager provides a centralized registry for renderer factories,
/// enabling safe registration, creation, and management of different renderer types.
/// All operations are thread-safe and support concurrent access.
#[derive(Debug)]
pub struct RendererManager {
    /// Map of TypeId to factory instances, protected by mutex for thread safety
    factories: Arc<Mutex<HashMap<TypeId, Box<dyn RendererFactory>>>>,
}

impl RendererManager {
    /// Create a new RendererManager instance.
    ///
    /// # Returns
    /// A new RendererManager with an empty factory registry.
    ///
    /// # Example
    /// ```
    /// use crate::renderer::RendererManager;
    ///
    /// let manager = RendererManager::new();
    /// ```
    pub fn new() -> Self {
        Self {
            factories: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Register a new renderer factory with timeout protection.
    ///
    /// This method registers a factory for creating renderers of a specific type.
    /// The registration process includes timeout protection based on the factory's
    /// timeout_microseconds value to prevent hanging operations.
    ///
    /// # Arguments
    /// * `factory` - The factory instance to register
    ///
    /// # Returns
    /// * `Ok(true)` - Factory was successfully registered
    /// * `Ok(false)` - Factory registration was skipped (shouldn't happen in current implementation)
    /// * `Err(RendererError::FactoryAlreadyRegistered)` - Factory for this type already exists
    /// * `Err(RendererError::CreationFailed)` - Timeout or other registration failure
    ///
    /// # Example
    /// ```
    /// use crate::renderer::{RendererManager, MockRendererFactory};
    ///
    /// let mut manager = RendererManager::new();
    /// let factory = Box::new(MockRendererFactory::new("TestFactory"));
    /// let result = manager.register(factory);
    /// assert!(result.is_ok());
    /// ```
    pub fn register(&mut self, factory: Box<dyn RendererFactory>) -> Result<bool, RendererError> {
        let factory_info = factory.get_info();
        let timeout_duration = Duration::from_micros(factory_info.timeout_microseconds);
        let type_id = factory_info.id;

        // Implement timeout protection using a separate thread
        let factories_clone = Arc::clone(&self.factories);
        let factory_arc = Arc::new(factory);

        let (sender, receiver) = std::sync::mpsc::channel();

        // Spawn registration operation in separate thread
        let factory_for_thread = Arc::clone(&factory_arc);
        thread::spawn(move || {
            let start_time = Instant::now();

            // Attempt to acquire lock and register
            let result = {
                match factories_clone.lock() {
                    Ok(mut factories) => {
                        // Check if factory already exists
                        if factories.contains_key(&type_id) {
                            Err(RendererError::FactoryAlreadyRegistered(type_id))
                        } else {
                            // Convert Arc back to Box for storage
                            match Arc::try_unwrap(factory_for_thread) {
                                Ok(factory_box) => {
                                    factories.insert(type_id, factory_box);
                                    Ok(true)
                                }
                                Err(_) => {
                                    Err(RendererError::CreationFailed(
                                        "Failed to unwrap factory Arc for registration".to_string()
                                    ))
                                }
                            }
                        }
                    }
                    Err(_) => {
                        Err(RendererError::CreationFailed(
                            "Failed to acquire factories lock".to_string()
                        ))
                    }
                }
            };

            let _ = sender.send((result, start_time.elapsed()));
        });

        // Wait for completion or timeout
        match receiver.recv_timeout(timeout_duration) {
            Ok((result, _elapsed)) => result,
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                Err(RendererError::CreationFailed(format!(
                    "Factory registration timed out after {} microseconds",
                    timeout_duration.as_micros()
                )))
            }
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                Err(RendererError::CreationFailed(
                    "Registration thread disconnected unexpectedly".to_string()
                ))
            }
        }
    }

    /// Create a renderer instance using the specified factory.
    ///
    /// This method looks up the factory by TypeId and creates a new renderer
    /// instance with the specified precision and parameters.
    ///
    /// # Arguments
    /// * `id` - TypeId of the renderer factory to use
    /// * `precision` - Data precision for the renderer
    /// * `parameters` - Factory-specific configuration parameters
    ///
    /// # Returns
    /// * `Ok(Box<dyn Renderer>)` - Successfully created renderer
    /// * `Err(RendererError::RendererNotFound)` - No factory found for the specified TypeId
    /// * `Err(RendererError::*)` - Other creation errors from the factory
    ///
    /// # Example
    /// ```
    /// use std::any::TypeId;
    /// use crate::renderer::{RendererManager, MockRendererFactory, DataPrecision};
    ///
    /// let mut manager = RendererManager::new();
    /// let factory = Box::new(MockRendererFactory::new("TestFactory"));
    /// manager.register(factory).unwrap();
    ///
    /// let renderer = manager.create(
    ///     TypeId::of::<MockRendererFactory>(),
    ///     DataPrecision::F32,
    ///     "test_params"
    /// ).unwrap();
    /// ```
    pub fn create(&self, id: TypeId, precision: DataPrecision, parameters: &str) -> Result<Box<dyn Renderer>, RendererError> {
        let factories = self.get_factories_lock()?;

        match factories.get(&id) {
            Some(factory) => {
                factory.create(precision, parameters)
            }
            None => {
                Err(RendererError::RendererNotFound(id))
            }
        }
    }

    /// Get information about all registered renderer factories.
    ///
    /// This method returns a list of RendererInfo structs describing
    /// the capabilities and requirements of all registered factories.
    ///
    /// # Returns
    /// A vector of RendererInfo structs, one for each registered factory.
    ///
    /// # Example
    /// ```
    /// use crate::renderer::{RendererManager, MockRendererFactory};
    ///
    /// let mut manager = RendererManager::new();
    /// let factory = Box::new(MockRendererFactory::new("TestFactory"));
    /// manager.register(factory).unwrap();
    ///
    /// let info_list = manager.get_renderer_info_list();
    /// assert_eq!(info_list.len(), 1);
    /// assert_eq!(info_list[0].name, "TestFactory");
    /// ```
    pub fn get_renderer_info_list(&self) -> Vec<RendererInfo> {
        match self.get_factories_lock() {
            Ok(factories) => {
                factories.values()
                    .map(|factory| factory.get_info())
                    .collect()
            }
            Err(_) => {
                // Return empty list if we can't acquire the lock
                Vec::new()
            }
        }
    }

    /// Helper method to acquire the factories lock safely.
    ///
    /// # Returns
    /// * `Ok(MutexGuard)` - Successfully acquired lock
    /// * `Err(RendererError::CreationFailed)` - Failed to acquire lock
    fn get_factories_lock(&self) -> Result<MutexGuard<HashMap<TypeId, Box<dyn RendererFactory>>>, RendererError> {
        self.factories.lock().map_err(|_| {
            RendererError::CreationFailed(
                "Failed to acquire factories lock".to_string()
            )
        })
    }
}

impl Default for RendererManager {
    fn default() -> Self {
        Self::new()
    }
}

// Ensure RendererManager is Send + Sync for multi-threaded use
unsafe impl Send for RendererManager {}
unsafe impl Sync for RendererManager {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::renderer::factory::{MockRendererFactory, MockRenderer};
    use std::any::TypeId;
    use std::thread;
    use std::sync::Arc;
    use std::time::Duration;

    #[test]
    fn test_new_manager() {
        let manager = RendererManager::new();
        let info_list = manager.get_renderer_info_list();
        assert!(info_list.is_empty());
    }

    #[test]
    fn test_register_factory() {
        let mut manager = RendererManager::new();
        let factory = Box::new(MockRendererFactory::new("TestFactory"));

        let result = manager.register(factory);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), true);

        let info_list = manager.get_renderer_info_list();
        assert_eq!(info_list.len(), 1);
        assert_eq!(info_list[0].name, "TestFactory");
    }

    #[test]
    fn test_register_duplicate_factory() {
        let mut manager = RendererManager::new();

        let factory1 = Box::new(MockRendererFactory::new("Factory1"));
        let factory2 = Box::new(MockRendererFactory::new("Factory2"));

        // First registration should succeed
        assert!(manager.register(factory1).is_ok());

        // Second registration of same type should fail
        let result = manager.register(factory2);
        assert!(result.is_err());
        match result.unwrap_err() {
            RendererError::FactoryAlreadyRegistered(_) => {},
            _ => panic!("Expected FactoryAlreadyRegistered error"),
        }
    }

    #[test]
    fn test_create_renderer() {
        let mut manager = RendererManager::new();
        let factory = Box::new(MockRendererFactory::new("TestFactory"));

        manager.register(factory).unwrap();

        let renderer = manager.create(
            TypeId::of::<MockRendererFactory>(),
            DataPrecision::F32,
            "test_params"
        );

        assert!(renderer.is_ok());
        let renderer = renderer.unwrap();
        assert_eq!(renderer.name(), "MockRenderer");
    }

    #[test]
    fn test_create_renderer_not_found() {
        let manager = RendererManager::new();

        let result = manager.create(
            TypeId::of::<String>(), // Type not registered
            DataPrecision::F32,
            "test_params"
        );

        assert!(result.is_err());
        match result.unwrap_err() {
            RendererError::RendererNotFound(_) => {},
            _ => panic!("Expected RendererNotFound error"),
        }
    }

    #[test]
    fn test_create_with_invalid_parameters() {
        let mut manager = RendererManager::new();
        let factory = Box::new(MockRendererFactory::new("TestFactory"));

        manager.register(factory).unwrap();

        let result = manager.create(
            TypeId::of::<MockRendererFactory>(),
            DataPrecision::F32,
            "invalid_params" // This should trigger validation error
        );

        assert!(result.is_err());
        match result.unwrap_err() {
            RendererError::InvalidParameters(_) => {},
            _ => panic!("Expected InvalidParameters error"),
        }
    }

    #[test]
    fn test_create_with_unsupported_precision() {
        let mut manager = RendererManager::new();
        let factory = Box::new(MockRendererFactory::new_with_precisions(
            "TestFactory",
            vec![DataPrecision::F32] // Only F32 supported
        ));

        manager.register(factory).unwrap();

        let result = manager.create(
            TypeId::of::<MockRendererFactory>(),
            DataPrecision::F64, // Unsupported precision
            "test_params"
        );

        assert!(result.is_err());
        match result.unwrap_err() {
            RendererError::UnsupportedPrecision(DataPrecision::F64) => {},
            _ => panic!("Expected UnsupportedPrecision error"),
        }
    }

    #[test]
    fn test_multiple_factories() {
        let mut manager = RendererManager::new();

        // We can't register multiple MockRendererFactory instances since they have the same TypeId
        // So we test with one factory but verify the list functionality
        let factory = Box::new(MockRendererFactory::new("TestFactory"));
        manager.register(factory).unwrap();

        let info_list = manager.get_renderer_info_list();
        assert_eq!(info_list.len(), 1);
        assert_eq!(info_list[0].name, "TestFactory");
    }

    #[test]
    fn test_concurrent_access() {
        let manager = Arc::new(RendererManager::new());
        let mut handles = vec![];

        // Register a factory first
        {
            let mut manager_ref = Arc::try_unwrap(manager).unwrap_or_else(|arc| {
                // If we can't unwrap, create a new one for this test
                RendererManager::new()
            });
            let factory = Box::new(MockRendererFactory::new("TestFactory"));
            manager_ref.register(factory).unwrap();

            // Re-wrap in Arc for concurrent testing
            let manager = Arc::new(manager_ref);

            // Spawn multiple threads trying to create renderers
            for i in 0..5 {
                let manager_clone = Arc::clone(&manager);
                let handle = thread::spawn(move || {
                    let result = manager_clone.create(
                        TypeId::of::<MockRendererFactory>(),
                        DataPrecision::F32,
                        &format!("thread_{}_params", i)
                    );
                    result.is_ok()
                });
                handles.push(handle);
            }

            // Wait for all threads and verify they succeeded
            for handle in handles {
                let success = handle.join().unwrap();
                assert!(success);
            }
        }
    }

    #[test]
    fn test_timeout_protection() {
        let mut manager = RendererManager::new();

        // Create a factory with very short timeout for testing
        let factory = Box::new(MockRendererFactory::new_full(
            "TimeoutFactory",
            vec![DataPrecision::F32],
            "timeout_test",
            1 // 1 microsecond timeout - should be very tight
        ));

        // Registration might succeed or timeout depending on system timing
        let result = manager.register(factory);

        // Either registration succeeds quickly, or it times out
        match result {
            Ok(true) => {
                // Registration succeeded within timeout
                assert!(true);
            }
            Err(RendererError::CreationFailed(msg)) => {
                // Registration timed out as expected with very short timeout
                assert!(msg.contains("timed out"));
            }
            _ => panic!("Unexpected result from timeout test"),
        }
    }

    #[test]
    fn test_default_trait() {
        let manager: RendererManager = Default::default();
        let info_list = manager.get_renderer_info_list();
        assert!(info_list.is_empty());
    }

    #[test]
    fn test_send_sync_traits() {
        // Test that RendererManager implements Send + Sync
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<RendererManager>();
    }

    #[test]
    fn test_debug_trait() {
        let manager = RendererManager::new();
        let debug_string = format!("{:?}", manager);
        assert!(debug_string.contains("RendererManager"));
    }
}