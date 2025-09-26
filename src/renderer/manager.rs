use std::any::TypeId;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard, mpsc};
use std::time::Duration;
use std::thread;
use crate::renderer::{DataPrecision, RendererError, RendererFactory, RendererInfo, Renderer};

/// Factory registry with automatic renderer cleanup.
///
/// The RendererManager's responsibilities:
/// 1. Register renderer factories
/// 2. Create renderer instances on demand
/// 3. Provide factory information for capability discovery
/// 4. **Automatically clean up all created renderers when destroyed with timeout handling**
///
/// **IMPORTANT**: The Renderer trait must implement:
///
/// Day-to-day management (start, communication, events) is handled by:
/// - The client code that gets the renderer
/// - The async_communication module
#[derive(Debug)]
pub struct RendererManager {
    /// Map of TypeId to factory instances, protected by mutex for thread safety
    factories: Arc<Mutex<HashMap<TypeId, Box<dyn RendererFactory>>>>,
    /// Shutdown channels for all created renderers - used for cleanup
    /// Each entry contains: (renderer_id, shutdown_sender, confirmation_receiver, timeout_duration)
    renderer_shutdowns: Mutex<Vec<(u64, mpsc::Sender<()>, mpsc::Receiver<()>, Duration)>>,
}

impl RendererManager {
    pub fn new() -> Self {
        Self {
            factories: Arc::new(Mutex::new(HashMap::new())),
            renderer_shutdowns: Mutex::new(Vec::new()),
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
    /// use fulgor::renderer::manager::RendererManager;
    /// use fulgor::renderer::factory::MockRendererFactory;
    ///
    /// let mut manager = RendererManager::new();
    /// let factory = Box::new(MockRendererFactory::new("TestFactory"));
    /// let result = manager.register(factory);
    /// assert!(result.is_ok());
    /// ```
    pub fn register(&mut self, factory: Box<dyn RendererFactory>) -> Result<bool, RendererError> {
        let factory_info = factory.get_info();
        let timeout_duration = Duration::from_micros(factory_info.timeout_microseconds);

        // FIX: Use the factory's actual TypeId (since RendererFactory extends Any)
        let type_id = factory.as_ref().type_id();  // ← Get actual factory TypeId from trait object

        // Implement timeout protection using a separate thread
        let factories_clone = Arc::clone(&self.factories);
        let factory_arc = Arc::new(factory);

        let (sender, receiver) = mpsc::channel();

        thread::spawn(move || {
            let start_time = std::time::Instant::now();

            let result = {
                match factories_clone.lock() {
                    Ok(mut factories) => {
                        if factories.contains_key(&type_id) {
                            Err(RendererError::FactoryAlreadyRegistered(type_id))
                        } else {
                            match Arc::try_unwrap(factory_arc) {
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

        match receiver.recv_timeout(timeout_duration) {
            Ok((result, _elapsed)) => result,
            Err(mpsc::RecvTimeoutError::Timeout) => {
                Err(RendererError::CreationFailed(format!(
                    "Factory registration timed out after {} microseconds",
                    timeout_duration.as_micros()
                )))
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
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
    /// use fulgor::renderer::manager::RendererManager;
    /// use fulgor::renderer::factory::MockRendererFactory;
    /// use fulgor::renderer::DataPrecision;
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
    pub fn create(
        &self,
        type_id: TypeId,
        precision: DataPrecision,
        parameters: &str,
    ) -> Result<(Box<dyn Renderer>, mpsc::Receiver<()>, mpsc::Sender<()>), RendererError> {
        let renderer = {
            let factories = self.get_factories_lock()?;

            if let Some(factory) = factories.get(&type_id) {
                factory.create(precision, parameters)?
            } else {
                return Err(RendererError::RendererNotFound(type_id));
            }
        };

        self.setup_renderer_tracking(renderer)
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
    /// use fulgor::renderer::manager::RendererManager;
    /// use fulgor::renderer::factory::MockRendererFactory;
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
            Err(_) => Vec::new(),
        }
    }

    /// Create a renderer instance by factory name instead of TypeId.
    ///
    /// This method provides a more user-friendly way to create renderers by using
    /// human-readable names instead of TypeIds. It searches through all registered
    /// factories to find one with the matching name.
    ///
    /// # Arguments
    /// * `name` - Human-readable name of the renderer factory
    /// * `precision` - Data precision for the renderer
    /// * `parameters` - Factory-specific configuration parameters
    ///
    /// # Returns
    /// * `Ok(Box<dyn Renderer>)` - Successfully created renderer
    /// * `Err(RendererError::RendererNotFoundByName)` - No factory found with the specified name
    /// * `Err(RendererError::*)` - Other creation errors from the factory
    ///
    /// # Example
    /// ```
    /// use fulgor::renderer::manager::RendererManager;
    /// use fulgor::renderer::DataPrecision;
    ///
    /// let manager = RendererManager::new();
    /// let renderer = manager.create_by_name(
    ///     "MockRenderer",
    ///     DataPrecision::F32,
    ///     "test_params"
    /// );
    /// ```
    pub fn create_by_name(
        &self,
        name: &str,
        precision: DataPrecision,
        parameters: &str,
    ) -> Result<(Box<dyn Renderer>, mpsc::Receiver<()>, mpsc::Sender<()>), RendererError> {
        let factories = self.get_factories_lock()?;

        for factory in factories.values() {
            if factory.get_info().name == name {
                let renderer = factory.create(precision, parameters)?;
                drop(factories); // Release the lock before calling setup_renderer_tracking
                return self.setup_renderer_tracking(renderer);
            }
        }

        Err(RendererError::RendererNotFoundByName(name.to_string()))
    }

    /// Find all renderer factories that support a specific capability.
    ///
    /// This method searches through all registered factories and returns information
    /// about those that support the specified capability. The capability matching
    /// is case-sensitive and matches against trimmed capability names.
    ///
    /// # Arguments
    /// * `capability` - The capability to search for (e.g., "3d_rendering", "gpu_accelerated")
    ///
    /// # Returns
    /// A vector of RendererInfo structs for factories that support the capability.
    /// Returns an empty vector if no factories support the capability.
    ///
    /// # Example
    /// ```
    /// use fulgor::renderer::manager::RendererManager;
    ///
    /// let manager = RendererManager::new();
    /// let gpu_renderers = manager.find_by_capability("gpu_accelerated");
    /// for renderer_info in gpu_renderers {
    ///     println!("GPU renderer: {}", renderer_info.name);
    /// }
    /// ```
    pub fn find_by_capability(&self, capability: &str) -> Vec<RendererInfo> {
        match self.get_factories_lock() {
            Ok(factories) => {
                factories
                    .values()
                    .map(|factory| factory.get_info())
                    .filter(|info| info.has_capability(capability))
                    .collect()
            }
            Err(_) => {
                // Return empty list if we can't acquire the lock
                Vec::new()
            }
        }
    }

    /// Find all renderer factories that support a specific data precision.
    ///
    /// This method searches through all registered factories and returns information
    /// about those that can create renderers with the specified data precision.
    /// It tests precision support by attempting parameter validation.
    ///
    /// # Arguments
    /// * `precision` - The data precision to search for
    ///
    /// # Returns
    /// A vector of RendererInfo structs for factories that support the precision.
    /// Returns an empty vector if no factories support the precision.
    ///
    /// # Example
    /// ```
    /// use fulgor::renderer::manager::RendererManager;
    /// use fulgor::renderer::DataPrecision;
    ///
    /// let manager = RendererManager::new();
    /// let f64_renderers = manager.find_by_precision(DataPrecision::F64);
    /// for renderer_info in f64_renderers {
    ///     println!("F64 renderer: {}", renderer_info.name);
    /// }
    /// ```
    pub fn find_by_precision(&self, precision: DataPrecision) -> Vec<RendererInfo> {
        match self.get_factories_lock() {
            Ok(factories) => {
                factories
                    .values()
                    .filter_map(|factory| {
                        // Test if factory supports this precision by attempting validation
                        // Use empty parameters for the test
                        match factory.validate_parameters(precision, "") {
                            Ok(_) => Some(factory.get_info()),
                            Err(RendererError::UnsupportedPrecision(_)) => None,
                            Err(_) => {
                                // Other errors (like InvalidParameters) suggest precision is supported
                                // but parameters are invalid, so include this factory
                                Some(factory.get_info())
                            }
                        }
                    })
                    .collect()
            }
            Err(_) => {
                // Return empty list if we can't acquire the lock
                Vec::new()
            }
        }
    }

    /// Set up shutdown tracking for a renderer
    fn setup_renderer_tracking(
        &self,
        renderer: Box<dyn Renderer>
    ) -> Result<(Box<dyn Renderer>, mpsc::Receiver<()>, mpsc::Sender<()>), RendererError> {
        // Get the shutdown timeout and unique ID from the renderer
        let shutdown_timeout = renderer.shutdown_timeout();
        let renderer_id = renderer.unique_id();

        // Create shutdown signal channel (manager -> renderer)
        let (shutdown_tx, shutdown_rx) = mpsc::channel();

        // Create confirmation channel (renderer -> manager)
        let (confirm_tx, confirm_rx) = mpsc::channel();

        // Track this renderer for cleanup
        self.renderer_shutdowns.lock().unwrap().push((renderer_id, shutdown_tx, confirm_rx, shutdown_timeout));

        Ok((renderer, shutdown_rx, confirm_tx))
    }

    /// Get the number of registered renderer factories.
    ///
    /// This method returns the count of currently registered factories in the manager.
    /// It's useful for monitoring and debugging the factory registration state.
    ///
    /// # Returns
    /// The number of registered factories, or 0 if the lock cannot be acquired.
    ///
    /// # Example
    /// ```
    /// use fulgor::renderer::manager::RendererManager;
    ///
    /// let manager = RendererManager::new();
    /// println!("Registered factories: {}", manager.get_factory_count());
    /// ```
    pub fn get_factory_count(&self) -> usize {
        self.factories.lock().map(|f| f.len()).unwrap_or(0)
    }

    /// Get the number of currently tracked renderers
    pub fn active_renderer_count(&self) -> usize {
        self.renderer_shutdowns.lock().map(|r| r.len()).unwrap_or(0)
    }

    /// Validate parameters for a specific factory without creating a renderer.
    ///
    /// This method allows checking if parameters are valid for a specific factory
    /// identified by name before attempting to create an expensive renderer instance.
    /// It's useful for early validation in user interfaces or configuration systems.
    ///
    /// # Arguments
    /// * `factory_name` - Name of the factory to validate parameters for
    /// * `precision` - Data precision to validate with
    /// * `parameters` - Parameters to validate
    ///
    /// # Returns
    /// * `Ok(())` - Parameters are valid for the specified factory
    /// * `Err(RendererError::RendererNotFoundByName)` - No factory found with the specified name
    /// * `Err(RendererError::UnsupportedPrecision)` - Factory doesn't support the precision
    /// * `Err(RendererError::InvalidParameters)` - Parameters are invalid for the factory
    /// * `Err(RendererError::CreationFailed)` - Failed to acquire factory lock
    ///
    /// # Example
    /// ```
    /// use fulgor::renderer::manager::RendererManager;
    /// use fulgor::renderer::DataPrecision;
    ///
    /// let manager = RendererManager::new();
    /// match manager.validate_parameters_for("MockRenderer", DataPrecision::F32, "test=true") {
    ///     Ok(_) => println!("Parameters are valid"),
    ///     Err(e) => println!("Validation failed: {}", e),
    /// }
    /// ```
    pub fn validate_parameters_for(
        &self,
        factory_name: &str,
        precision: DataPrecision,
        parameters: &str,
    ) -> Result<(), RendererError> {
        let factories = self.get_factories_lock()?;

        for factory in factories.values() {
            if factory.get_info().name == factory_name {
                return factory.validate_parameters(precision, parameters);
            }
        }

        Err(RendererError::RendererNotFoundByName(factory_name.to_string()))
    }

    /// Helper method to acquire the factories lock safely
    fn get_factories_lock(&self) -> Result<MutexGuard<'_, HashMap<TypeId, Box<dyn RendererFactory>>>, RendererError> {
        self.factories.lock().map_err(|_| {
            RendererError::CreationFailed(
                "Failed to acquire factories lock".to_string()
            )
        })
    }
    /// Find renderer factory information by name.
    ///
    /// This is a helper method that returns the RendererInfo for a factory
    /// with the specified name, if it exists.
    ///
    /// # Arguments
    /// * `name` - Name of the factory to find
    ///
    /// # Returns
    /// * `Some(RendererInfo)` - Information about the factory
    /// * `None` - No factory found with the specified name
    ///
    /// # Example
    /// ```
    /// use fulgor::renderer::manager::RendererManager;
    ///
    /// let manager = RendererManager::new();
    /// if let Some(info) = manager.find_factory_by_name("MockRenderer") {
    ///     println!("Found factory: {} with {} capabilities",
    ///              info.name, info.get_capabilities().len());
    /// }
    /// ```
    pub fn find_factory_by_name(&self, name: &str) -> Option<RendererInfo> {
        match self.get_factories_lock() {
            Ok(factories) => {
                factories
                    .values()
                    .map(|factory| factory.get_info())
                    .find(|info| info.name == name)
            }
            Err(_) => None,
        }
    }

    /// Get a list of all supported capabilities across all registered factories.
    ///
    /// This method aggregates all unique capabilities from all registered factories,
    /// providing a comprehensive view of what the renderer system can do.
    ///
    /// # Returns
    /// A sorted vector of unique capability strings across all factories.
    ///
    /// # Example
    /// ```
    /// use fulgor::renderer::manager::RendererManager;
    ///
    /// let manager = RendererManager::new();
    /// let all_capabilities = manager.get_all_capabilities();
    /// println!("System supports: {:?}", all_capabilities);
    /// ```
    pub fn get_all_capabilities(&self) -> Vec<String> {
        use std::collections::BTreeSet;

        match self.get_factories_lock() {
            Ok(factories) => {
                let mut capabilities_set = BTreeSet::new();

                for factory in factories.values() {
                    let info = factory.get_info();
                    for capability in info.get_capabilities() {
                        capabilities_set.insert(capability.to_string());
                    }
                }

                capabilities_set.into_iter().collect()
            }
            Err(_) => Vec::new(),
        }
    }

    /// Get a list of all supported data precisions across all registered factories.
    ///
    /// This method determines which data precisions are supported by at least
    /// one registered factory, providing insight into the system's precision capabilities.
    ///
    /// # Returns
    /// A vector of DataPrecision variants supported by at least one factory.
    ///
    /// # Example
    /// ```
    /// use fulgor::renderer::manager::RendererManager;
    ///
    /// let manager = RendererManager::new();
    /// let supported_precisions = manager.get_supported_precisions();
    /// println!("Supported precisions: {:?}", supported_precisions);
    /// ```
    pub fn get_supported_precisions(&self) -> Vec<DataPrecision> {
        use std::collections::BTreeSet;

        let all_precisions = [
            DataPrecision::F16,
            DataPrecision::F32,
            DataPrecision::F64,
            DataPrecision::BFloat16,
        ];

        let mut supported_set = BTreeSet::new();

        for precision in &all_precisions {
            if !self.find_by_precision(*precision).is_empty() {
                supported_set.insert(*precision);
            }
        }

        supported_set.into_iter().collect()
    }
}

impl Default for RendererManager {
    fn default() -> Self {
        Self::new()
    }
}

// **AUTOMATIC CLEANUP WITH TIMEOUT HANDLING** - Pure std library!
impl Drop for RendererManager {
    fn drop(&mut self) {
        println!("RendererManager dropping - initiating shutdown of all renderers...");

        if let Ok(mut shutdowns) = self.renderer_shutdowns.lock() {
            let renderer_count = shutdowns.len();
            if renderer_count == 0 {
                println!("No active renderers to shut down.");
                return;
            }

            // Send shutdown signals to all renderers
            let mut confirmations = Vec::new();
            for (renderer_id, shutdown_tx, confirm_rx, timeout) in shutdowns.drain(..) {
                // Send shutdown signal (ignore if receiver already dropped)
                if shutdown_tx.send(()).is_ok() {
                    println!("Sent shutdown signal to renderer {}", renderer_id);
                }
                confirmations.push((renderer_id, confirm_rx, timeout));
            }

            println!("Sent shutdown signals to {} renderers, waiting for confirmations...", renderer_count);

            // Wait for confirmations with individual timeouts using std library
            let mut completed = 0;
            let mut timed_out = 0;

            for (renderer_id, confirm_rx, timeout) in confirmations {
                match confirm_rx.recv_timeout(timeout) {
                    Ok(()) => {
                        println!("Renderer {} confirmed shutdown", renderer_id);
                        completed += 1;
                    }
                    Err(mpsc::RecvTimeoutError::Timeout) => {
                        timed_out += 1;
                        eprintln!("WARNING: Renderer {} did not confirm shutdown within {:?} timeout",
                                  renderer_id, timeout);
                    }
                    Err(mpsc::RecvTimeoutError::Disconnected) => {
                        // Channel was dropped, renderer probably stopped
                        println!("Renderer {} shutdown (channel disconnected)", renderer_id);
                        completed += 1;
                    }
                }
            }

            println!("Renderer shutdown complete: {} confirmed, {} timed out", completed, timed_out);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::renderer::factory::{MockRendererFactory};
    use std::thread;
    use std::sync::Arc;
    use std::time::Duration;

    fn create_single_test_factory_manager() -> RendererManager {
        let mut manager = RendererManager::new();
        let factory = Box::new(MockRendererFactory::new_full(
            "TestRenderer",
            vec![DataPrecision::F16, DataPrecision::F32, DataPrecision::F64],
            "cpu_rendering,gpu_rendering,basic_3d,real_time",
            5000,
        ));
        manager.register(factory).unwrap();
        manager
    }

    // For tests that really need multiple factories, use this pattern with wrapper types:
    fn create_multi_factory_manager() -> RendererManager {
        let mut manager = RendererManager::new();

        // Different wrapper types, each with unique TypeId
        #[derive(Debug)]
        struct CpuFactory(MockRendererFactory);
        impl RendererFactory for CpuFactory {
            fn create(&self, precision: DataPrecision, parameters: &str) -> Result<Box<dyn Renderer>, RendererError> {
                self.0.create(precision, parameters)
            }
            fn get_info(&self) -> RendererInfo { self.0.get_info() }
            fn validate_parameters(&self, precision: DataPrecision, parameters: &str) -> Result<(), RendererError> {
                self.0.validate_parameters(precision, parameters)
            }
        }

        #[derive(Debug)]
        struct GpuFactory(MockRendererFactory);
        impl RendererFactory for GpuFactory {
            fn create(&self, precision: DataPrecision, parameters: &str) -> Result<Box<dyn Renderer>, RendererError> {
                self.0.create(precision, parameters)
            }
            fn get_info(&self) -> RendererInfo { self.0.get_info() }
            fn validate_parameters(&self, precision: DataPrecision, parameters: &str) -> Result<(), RendererError> {
                self.0.validate_parameters(precision, parameters)
            }
        }

        // Register different factory types
        let cpu_factory = Box::new(CpuFactory(MockRendererFactory::new_full(
            "CpuRenderer",
            vec![DataPrecision::F32, DataPrecision::F64],
            "cpu_rendering,basic_3d,software",
            3000,
        )));
        manager.register(cpu_factory).unwrap();

        let gpu_factory = Box::new(GpuFactory(MockRendererFactory::new_full(
            "GpuRenderer",
            vec![DataPrecision::F16, DataPrecision::F32, DataPrecision::BFloat16],
            "gpu_rendering,advanced_3d,hardware_accelerated,real_time",
            5000,
        )));
        manager.register(gpu_factory).unwrap();

        manager
    }

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
        let _renderer = renderer.unwrap();
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
        // Create manager and register factory BEFORE wrapping in Arc
        let mut manager = RendererManager::new();
        let factory = Box::new(MockRendererFactory::new("TestFactory"));
        manager.register(factory).unwrap();

        // Now wrap in Arc for concurrent testing
        let manager = Arc::new(manager);
        let mut handles = vec![];

        // Spawn multiple threads trying to create renderers
        for _i in 0..5 {
            let manager_clone = Arc::clone(&manager);
            let handle = thread::spawn(move || {
                // Use create_by_name instead of create with TypeId to avoid TypeId mismatch issues
                let result = manager_clone.create_by_name(
                    "TestFactory",  // Use the factory name instead of TypeId
                    DataPrecision::F32,
                    "" // Use empty parameters
                );

                // Return both success and error for debugging
                match result {
                    Ok(_) => (true, None),
                    Err(e) => (false, Some(format!("{:?}", e))),
                }
            });
            handles.push(handle);
        }

        // Wait for all threads and check results
        for (i, handle) in handles.into_iter().enumerate() {
            let (success, error) = handle.join().unwrap();
            if !success {
                panic!("Thread {} failed with error: {:?}", i, error.unwrap_or("Unknown error".to_string()));
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

    // Helper function to create a test manager with various factories
    fn create_test_manager_with_factories() -> RendererManager {
        let mut manager = RendererManager::new();

        // Register a basic CPU renderer
        let cpu_factory = Box::new(MockRendererFactory::new_full(
            "CpuRenderer",
            vec![DataPrecision::F32, DataPrecision::F64],
            "cpu_rendering,basic_3d,software",
            3000,
        ));
        manager.register(cpu_factory).unwrap();

        // Register a GPU renderer with different capabilities
        let gpu_factory = Box::new(MockRendererFactory::new_full(
            "GpuRenderer",
            vec![DataPrecision::F16, DataPrecision::F32, DataPrecision::BFloat16],
            "gpu_rendering,advanced_3d,hardware_accelerated,real_time",
            5000,
        ));
        manager.register(gpu_factory).unwrap();

        // Register a specialized high-precision renderer
        let precision_factory = Box::new(MockRendererFactory::new_full(
            "PrecisionRenderer",
            vec![DataPrecision::F64],
            "high_precision,scientific,cpu_rendering",
            10000,
        ));
        manager.register(precision_factory).unwrap();

        manager
    }

    #[test]
    fn test_single_factory_multiple_renderers() {
        let mut manager = RendererManager::new();

        // Register ONE factory type (as per design concept)
        let factory = Box::new(MockRendererFactory::new_full(
            "OpenGL3Renderer",
            vec![DataPrecision::F16, DataPrecision::F32, DataPrecision::F64],
            "gpu_rendering,hardware_accelerated,real_time",
            5000,
        ));
        manager.register(factory).unwrap();

        // Test that same factory can create multiple renderers with different parameters
        let renderer1 = manager.create_by_name("OpenGL3Renderer", DataPrecision::F16, "vsync=true").unwrap();
        let renderer2 = manager.create_by_name("OpenGL3Renderer", DataPrecision::F32, "vsync=false").unwrap();
        let renderer3 = manager.create_by_name("OpenGL3Renderer", DataPrecision::F64, "msaa=4").unwrap();

        // Verify they're different instances but from same factory
        assert_ne!(renderer1.0.unique_id(), renderer2.0.unique_id());
        assert_ne!(renderer2.0.unique_id(), renderer3.0.unique_id());

        // Verify they have the expected precisions
        assert_eq!(renderer1.0.get_data_precision(), DataPrecision::F16);
        assert_eq!(renderer2.0.get_data_precision(), DataPrecision::F32);
        assert_eq!(renderer3.0.get_data_precision(), DataPrecision::F64);
    }

    #[test]
    fn test_concurrent_access_single_factory() {
        let mut manager = RendererManager::new();

        // Register ONE factory (following design concept)
        let factory = Box::new(MockRendererFactory::new_full(
            "TestRenderer",
            vec![DataPrecision::F32, DataPrecision::F64],
            "cpu_rendering,software,basic_3d",
            3000,
        ));
        manager.register(factory).unwrap();

        let manager = Arc::new(manager);
        let mut handles = vec![];

        // Test concurrent access to the SINGLE factory
        for i in 0..10 {
            let manager_clone = Arc::clone(&manager);
            let handle = thread::spawn(move || {
                match i % 4 {
                    0 => {
                        // Test capability search
                        let _renderers = manager_clone.find_by_capability("cpu_rendering");
                    },
                    1 => {
                        // Test precision search
                        let _renderers = manager_clone.find_by_precision(DataPrecision::F32);
                    },
                    2 => {
                        // Test factory count
                        let _count = manager_clone.get_factory_count();
                    },
                    3 => {
                        // Test factory lookup
                        let _info = manager_clone.find_factory_by_name("TestRenderer");
                    },
                    _ => unreachable!(),
                }
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }
    }

    // CORRECT TEST APPROACH 3: Test Factory Uniqueness Enforcement
    #[test]
    fn test_factory_type_uniqueness_correctly_enforced() {
        let mut manager = RendererManager::new();

        // Register first MockRendererFactory
        let factory1 = Box::new(MockRendererFactory::new("FirstMock"));
        let result1 = manager.register(factory1);
        assert!(result1.is_ok()); // ✅ First registration succeeds

        // Try to register second MockRendererFactory (same TypeId)
        let factory2 = Box::new(MockRendererFactory::new("SecondMock"));
        let result2 = manager.register(factory2);
        assert!(result2.is_err()); // ✅ Correctly rejected!

        // Verify it's the right error type
        match result2.unwrap_err() {
            RendererError::FactoryAlreadyRegistered(_) => {}, // ✅ Expected
            _ => panic!("Wrong error type"),
        }

        // Verify only one factory is registered
        assert_eq!(manager.get_factory_count(), 1);
    }

    #[test]
    fn test_create_by_name_success() {
        let manager = create_single_test_factory_manager(); // Use single factory

        let renderer = manager.create_by_name("TestRenderer", DataPrecision::F32, "test");
        assert!(renderer.is_ok());
    }
    #[test]
    fn test_create_by_name_not_found() {
        let manager = create_single_test_factory_manager(); // Use single factory

        let result = manager.create_by_name("NonExistentRenderer", DataPrecision::F32, "test");
        assert!(result.is_err());

        match result.unwrap_err() {
            RendererError::RendererNotFoundByName(name) => {
                assert_eq!(name, "NonExistentRenderer");
            },
            _ => panic!("Expected RendererNotFoundByName error"),
        }
    }

    #[test]
    fn test_create_by_name_empty_registry() {
        let manager = RendererManager::new();

        let result = manager.create_by_name("AnyRenderer", DataPrecision::F32, "test");
        assert!(result.is_err());

        match result.unwrap_err() {
            RendererError::RendererNotFoundByName(_) => {},
            _ => panic!("Expected RendererNotFoundByName error"),
        }
    }

    #[test]
    fn test_create_by_name_unsupported_precision() {
        // Create manager with factory that has LIMITED precision support
        // This allows us to test the unsupported precision error case
        let mut manager = RendererManager::new();
        let factory = Box::new(MockRendererFactory::new_with_precisions(
            "LimitedRenderer",
            vec![DataPrecision::F32, DataPrecision::F64] // Only F32 and F64, NOT BFloat16
        ));
        manager.register(factory).unwrap();

        // Try to create renderer with unsupported BFloat16 precision
        let result = manager.create_by_name("LimitedRenderer", DataPrecision::BFloat16, "test");
        assert!(result.is_err());

        match result.unwrap_err() {
            RendererError::UnsupportedPrecision(DataPrecision::BFloat16) => {
                // ✅ Expected error - factory correctly rejected unsupported precision
            },
            _ => panic!("Expected UnsupportedPrecision error"),
        }
    }

    #[test]
    fn test_find_by_capability_success() {
        let manager = create_multi_factory_manager(); // Needs multiple for capability testing

        let cpu_renderers = manager.find_by_capability("cpu_rendering");
        assert_eq!(cpu_renderers.len(), 1);
        assert_eq!(cpu_renderers[0].name, "CpuRenderer");

        let gpu_renderers = manager.find_by_capability("gpu_rendering");
        assert_eq!(gpu_renderers.len(), 1);
        assert_eq!(gpu_renderers[0].name, "GpuRenderer");
    }

    #[test]
    fn test_find_by_capability_not_found() {
        // Create manager with factory that has specific capabilities
        let mut manager = RendererManager::new();
        let factory = Box::new(MockRendererFactory::new_full(
            "TestRenderer",
            vec![DataPrecision::F32, DataPrecision::F64],
            "cpu_rendering,basic_3d,software", // Has these capabilities
            3000,
        ));
        manager.register(factory).unwrap();

        // Search for non-existent capability - should return empty
        let result = manager.find_by_capability("quantum_rendering");
        assert!(result.is_empty());

        // Search for capability with different case - should return empty (case sensitive)
        let result = manager.find_by_capability("CPU_RENDERING");
        assert!(result.is_empty());
    }

    #[test]
    fn test_find_by_capability_empty_registry() {
        let manager = RendererManager::new();

        let result = manager.find_by_capability("any_capability");
        assert!(result.is_empty());
    }

    #[test]
    fn test_find_by_capability_whitespace_handling() {
        // Create manager with factory that has specific capabilities
        let mut manager = RendererManager::new();
        let factory = Box::new(MockRendererFactory::new_full(
            "TestRenderer",
            vec![DataPrecision::F32, DataPrecision::F64],
            "cpu_rendering,gpu_rendering,basic_3d", // Has these exact capabilities
            3000,
        ));
        manager.register(factory).unwrap();

        // Test that capability matching handles whitespace correctly
        let result = manager.find_by_capability(" cpu_rendering ");
        assert!(result.is_empty()); // Should not match due to leading/trailing spaces

        let result = manager.find_by_capability("cpu_rendering");
        assert_eq!(result.len(), 1); // Should match exactly
        assert_eq!(result[0].name, "TestRenderer");

        // Test other variations with whitespace
        let result = manager.find_by_capability(" gpu_rendering");
        assert!(result.is_empty()); // Leading space should not match

        let result = manager.find_by_capability("gpu_rendering ");
        assert!(result.is_empty()); // Trailing space should not match

        let result = manager.find_by_capability("gpu_rendering");
        assert_eq!(result.len(), 1); // Exact match should work
    }

    #[test]
    fn test_find_by_precision_success() {
        let manager = create_multi_factory_manager(); // Needs multiple for precision testing

        let f32_renderers = manager.find_by_precision(DataPrecision::F32);
        assert_eq!(f32_renderers.len(), 2); // Both CpuRenderer and GpuRenderer

        let f64_renderers = manager.find_by_precision(DataPrecision::F64);
        assert_eq!(f64_renderers.len(), 1); // Only CpuRenderer
        assert_eq!(f64_renderers[0].name, "CpuRenderer");
    }

    #[test]
    fn test_find_by_precision_not_found() {
        // Create a manager with limited precision support
        let mut manager = RendererManager::new();
        let factory = Box::new(MockRendererFactory::new_with_precisions(
            "LimitedRenderer",
            vec![DataPrecision::F32],
        ));
        manager.register(factory).unwrap();

        // Search for unsupported precision
        let result = manager.find_by_precision(DataPrecision::F64);
        assert!(result.is_empty());
    }

    #[test]
    fn test_find_by_precision_empty_registry() {
        let manager = RendererManager::new();

        let result = manager.find_by_precision(DataPrecision::F32);
        assert!(result.is_empty());
    }

    #[test]
    fn test_get_factory_count() {
        let manager = RendererManager::new();
        assert_eq!(manager.get_factory_count(), 0);

        let manager = create_test_manager_with_factories();
        assert_eq!(manager.get_factory_count(), 3);

        // Add another factory
        let mut manager = manager;
        let extra_factory = Box::new(MockRendererFactory::new("ExtraRenderer"));
        manager.register(extra_factory).unwrap();
        assert_eq!(manager.get_factory_count(), 4);
    }

    #[test]
    fn test_validate_parameters_for_success() {
        let manager = create_single_test_factory_manager(); // Single factory is enough

        let result = manager.validate_parameters_for("TestRenderer", DataPrecision::F32, "test");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_parameters_for_factory_not_found() {
        let manager = create_test_manager_with_factories();

        let result = manager.validate_parameters_for("NonExistentRenderer", DataPrecision::F32, "test");
        assert!(result.is_err());

        match result.unwrap_err() {
            RendererError::RendererNotFoundByName(name) => {
                assert_eq!(name, "NonExistentRenderer");
            },
            _ => panic!("Expected RendererNotFoundByName error"),
        }
    }

    #[test]
    fn test_validate_parameters_for_unsupported_precision() {
        let manager = create_test_manager_with_factories();

        // Try to validate with unsupported precision
        let result = manager.validate_parameters_for("CpuRenderer", DataPrecision::BFloat16, "test");
        assert!(result.is_err());

        match result.unwrap_err() {
            RendererError::UnsupportedPrecision(DataPrecision::BFloat16) => {},
            _ => panic!("Expected UnsupportedPrecision error"),
        }
    }

    #[test]
    fn test_validate_parameters_for_invalid_parameters() {
        let manager = create_test_manager_with_factories();

        // Use parameters that the mock factory will reject
        let result = manager.validate_parameters_for("CpuRenderer", DataPrecision::F32, "invalid_params");
        assert!(result.is_err());

        match result.unwrap_err() {
            RendererError::InvalidParameters(_) => {},
            _ => panic!("Expected InvalidParameters error"),
        }
    }

    #[test]
    fn test_find_factory_by_name_success() {
        let manager = create_single_test_factory_manager(); // Single factory is enough

        let info = manager.find_factory_by_name("TestRenderer");
        assert!(info.is_some());
        assert_eq!(info.unwrap().name, "TestRenderer");
    }

    #[test]
    fn test_find_factory_by_name_not_found() {
        let manager = create_test_manager_with_factories();

        let result = manager.find_factory_by_name("NonExistentRenderer");
        assert!(result.is_none());
    }

    #[test]
    fn test_find_factory_by_name_empty_registry() {
        let manager = RendererManager::new();

        let result = manager.find_factory_by_name("AnyRenderer");
        assert!(result.is_none());
    }

    #[test]
    fn test_get_all_capabilities() {
        let manager = create_multi_factory_manager(); // Needs multiple for capability aggregation

        let capabilities = manager.get_all_capabilities();
        let expected_caps = vec![
            "advanced_3d", "basic_3d", "cpu_rendering", "gpu_rendering",
            "hardware_accelerated", "real_time", "software"
        ];

        for expected in &expected_caps {
            assert!(capabilities.contains(&expected.to_string()),
                    "Missing capability: {}", expected);
        }
    }

    #[test]
    fn test_get_all_capabilities_empty_registry() {
        let manager = RendererManager::new();

        let capabilities = manager.get_all_capabilities();
        assert!(capabilities.is_empty());
    }

    #[test]
    fn test_get_all_capabilities_deduplication() {
        let mut manager = RendererManager::new();

        // Register two factories with overlapping capabilities
        let factory1 = Box::new(MockRendererFactory::new_full(
            "Renderer1",
            vec![DataPrecision::F32],
            "capability1,capability2,shared",
            1000,
        ));
        manager.register(factory1).unwrap();

        let factory2 = Box::new(MockRendererFactory::new_full(
            "Renderer2",
            vec![DataPrecision::F32],
            "capability2,capability3,shared",
            1000,
        ));
        manager.register(factory2).unwrap();

        let capabilities = manager.get_all_capabilities();
        assert_eq!(capabilities.len(), 4); // capability1, capability2, capability3, shared
        assert!(capabilities.contains(&"capability1".to_string()));
        assert!(capabilities.contains(&"capability2".to_string()));
        assert!(capabilities.contains(&"capability3".to_string()));
        assert!(capabilities.contains(&"shared".to_string()));
    }

    #[test]
    fn test_get_supported_precisions() {
        let manager = create_test_manager_with_factories();

        let precisions = manager.get_supported_precisions();

        // Should contain all precisions supported by at least one factory
        let expected_precisions = vec![
            DataPrecision::BFloat16, // GpuRenderer
            DataPrecision::F16,      // GpuRenderer
            DataPrecision::F32,      // CpuRenderer, GpuRenderer
            DataPrecision::F64,      // CpuRenderer, PrecisionRenderer
        ];

        assert_eq!(precisions.len(), expected_precisions.len());
        for expected_precision in &expected_precisions {
            assert!(precisions.contains(expected_precision),
                    "Missing precision: {:?}", expected_precision);
        }
    }

    #[test]
    fn test_get_supported_precisions_empty_registry() {
        let manager = RendererManager::new();

        let precisions = manager.get_supported_precisions();
        assert!(precisions.is_empty());
    }

    #[test]
    fn test_get_supported_precisions_limited() {
        let mut manager = RendererManager::new();

        // Register a factory with only F32 support
        let factory = Box::new(MockRendererFactory::new_with_precisions(
            "LimitedRenderer",
            vec![DataPrecision::F32],
        ));
        manager.register(factory).unwrap();

        let precisions = manager.get_supported_precisions();
        assert_eq!(precisions.len(), 1);
        assert!(precisions.contains(&DataPrecision::F32));
    }

    #[test]
    fn test_concurrent_access_to_query_methods() {
        // Create manager with single factory (following design concept)
        let mut manager = RendererManager::new();
        let factory = Box::new(MockRendererFactory::new_full(
            "CpuRenderer",  // Keep original name that test expects
            vec![DataPrecision::F16, DataPrecision::F32, DataPrecision::F64],
            "cpu_rendering,gpu_rendering,real_time,basic_3d",
            5000,
        ));
        manager.register(factory).unwrap();

        let manager = Arc::new(manager);
        let mut handles = vec![];

        // Test concurrent read access to various query methods
        for i in 0..10 {
            let manager_clone = Arc::clone(&manager);
            let handle = thread::spawn(move || {
                // Test different query methods
                match i % 4 {
                    0 => {
                        let _renderers = manager_clone.find_by_capability("cpu_rendering");
                    },
                    1 => {
                        let _renderers = manager_clone.find_by_precision(DataPrecision::F32);
                    },
                    2 => {
                        let _count = manager_clone.get_factory_count();
                    },
                    3 => {
                        let _info = manager_clone.find_factory_by_name("CpuRenderer");  // Matches factory name
                    },
                    _ => unreachable!(),
                }
            });
            handles.push(handle);
        }

        // Wait for all threads to complete
        for handle in handles {
            handle.join().unwrap();
        }
    }

    #[test]
    fn test_case_sensitive_name_matching() {
        let mut manager = RendererManager::new();
        let factory = Box::new(MockRendererFactory::new("TestRenderer"));
        manager.register(factory).unwrap();

        // Exact match should work
        let result = manager.create_by_name("TestRenderer", DataPrecision::F32, "test");
        assert!(result.is_ok());

        // Case mismatch should fail
        let result = manager.create_by_name("testrenderer", DataPrecision::F32, "test");
        assert!(result.is_err());

        let result = manager.create_by_name("TESTRENDERER", DataPrecision::F32, "test");
        assert!(result.is_err());
    }

    #[test]
    fn test_error_propagation_through_query_methods() {
        // Create manager with single factory - enough to test error propagation
        let mut manager = RendererManager::new();
        let factory = Box::new(MockRendererFactory::new_full(
            "TestRenderer",
            vec![DataPrecision::F32, DataPrecision::F64],
            "cpu_rendering,basic_3d",
            5000,
        ));
        manager.register(factory).unwrap();

        // Test that parameter validation errors are properly propagated
        let result = manager.validate_parameters_for("TestRenderer", DataPrecision::F32, "invalid_params");
        assert!(result.is_err());

        // Test that creation errors are properly propagated through create_by_name
        let result = manager.create_by_name("TestRenderer", DataPrecision::F32, "invalid_params");
        assert!(result.is_err());

        match result.unwrap_err() {
            RendererError::InvalidParameters(_) => {
                // ✅ Success! The InvalidParameters error from the factory
                // was properly propagated through the manager to the caller
            },
            _ => panic!("Expected InvalidParameters error to be propagated"),
        }
    }

    #[test]
    fn test_edge_cases_with_empty_capabilities() {
        let mut manager = RendererManager::new();

        // Register a factory with empty capabilities
        let factory = Box::new(MockRendererFactory::new_full(
            "MinimalRenderer",
            vec![DataPrecision::F32],
            "", // Empty capabilities
            1000,
        ));
        manager.register(factory).unwrap();

        // Searching for any capability should return empty
        let result = manager.find_by_capability("any_capability");
        assert!(result.is_empty());

        // get_all_capabilities should return empty
        let capabilities = manager.get_all_capabilities();
        assert!(capabilities.is_empty());

        // Other methods should still work
        assert_eq!(manager.get_factory_count(), 1);
        let info = manager.find_factory_by_name("MinimalRenderer");
        assert!(info.is_some());
    }

    #[test]
    fn test_performance_with_many_factories() {
        let mut manager = RendererManager::new();

        // Register many factories to test performance characteristics
        for i in 0..100 {
            let factory = Box::new(MockRendererFactory::new_full(
                format!("Renderer{}", i),
                vec![DataPrecision::F32],
                format!("capability{},shared", i),
                1000,
            ));
            manager.register(factory).unwrap();
        }

        assert_eq!(manager.get_factory_count(), 100);

        // Test query performance
        let start = std::time::Instant::now();
        let _shared_renderers = manager.find_by_capability("shared");
        let query_duration = start.elapsed();

        // Should complete quickly (this is more of a smoke test)
        assert!(query_duration < Duration::from_millis(100));

        // All factories should have the shared capability
        let shared_renderers = manager.find_by_capability("shared");
        assert_eq!(shared_renderers.len(), 100);
    }
}