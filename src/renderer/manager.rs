use std::any::TypeId;
use std::collections::HashMap;
use std::sync::{mpsc, Arc, Mutex, MutexGuard};
use std::sync::atomic::AtomicU64;
use std::time::{Duration, Instant};
use std::thread;

use crate::renderer::{BufferedAsyncSender, DataPrecision, RendererError, RendererEvent, RendererEventStream, RendererFactory, RendererInfo, RendererKind};
use crate::renderer::factory::Renderer;

/// Internal state of the manager.
#[derive(Debug)]
struct RendererManagerInner {
    renderers: HashMap<RendererKind, Box<dyn Renderer + Send + Sync>>,
    active: Option<RendererKind>,
    sender: Option<mpsc::Sender<RendererEvent>>,
    async_sinks: Vec<Arc<Mutex<Vec<RendererEvent>>>>,
    // New field for async buffered sender
    buffered_async_sender: Option<BufferedAsyncSender<RendererEvent>>,
}

/// Thread-safe manager for renderer factories with timeout protection.
///
/// The RendererManager provides a centralized registry for renderer factories,
/// enabling safe registration, creation, and management of different renderer types.
/// All operations are thread-safe and support concurrent access.
#[derive(Debug)]
pub struct RendererManager {
    /// Map of TypeId to factory instances, protected by mutex for thread safety
    factories: Arc<Mutex<HashMap<TypeId, Box<dyn RendererFactory>>>>,
    inner: Arc<Mutex<RendererManagerInner>>,
}

impl RendererManager {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(RendererManagerInner {
                renderers: HashMap::new(),
                active: None,
                sender: None,
                async_sinks: Vec::new(),
                buffered_async_sender: None,
            })),
            factories: Arc::new(Mutex::new(HashMap::new()))
        }
    }

    /// Subscribe synchronously (std channel).
    pub fn subscribe(&self) -> mpsc::Receiver<RendererEvent> {
        let (tx, rx) = mpsc::channel();
        let mut inner = self.inner.lock().unwrap();
        inner.sender = Some(tx);
        rx
    }

    /// Subscribe asynchronously (returns a Stream).
    pub fn async_subscribe(&self) -> RendererEventStream {
        let sink = Arc::new(Mutex::new(Vec::new()));
        {
            let mut inner = self.inner.lock().unwrap();
            inner.async_sinks.push(sink.clone());
        }
        RendererEventStream { buffer: sink }
    }

    /// Subscribe using BufferedAsyncSender with bounded channel.
    pub fn subscribe_buffered_bounded(
        &self,
        capacity: usize,
        drop_oldest_on_full: bool
    ) -> tokio::sync::mpsc::Receiver<RendererEvent> {
        let (buffered_sender, receiver) = BufferedAsyncSender::<RendererEvent>::new_bounded(capacity,drop_oldest_on_full,Arc::new(AtomicU64::new(0)));
        let mut inner = self.inner.lock().unwrap();
        inner.buffered_async_sender = Some(buffered_sender);
        receiver
    }

    /// Subscribe using BufferedAsyncSender with unbounded channel.
    pub fn subscribe_buffered_unbounded(&self) -> tokio::sync::mpsc::UnboundedReceiver<RendererEvent> {
        let (buffered_sender, receiver) = BufferedAsyncSender::<RendererEvent>::new_unbounded(Option::<usize>::Some(1));
        let mut inner = self.inner.lock().unwrap();
        inner.buffered_async_sender = Some(buffered_sender);
        receiver
    }

    /// Get the current BufferedAsyncSender if available.
    pub fn get_buffered_sender(&self) -> Option<BufferedAsyncSender<RendererEvent>> {
        let inner = self.inner.lock().unwrap();
        inner.buffered_async_sender.clone()
    }

    /// Notify all subscribers including the buffered async sender.
    async fn notify_async(&self, event: RendererEvent) {
        let inner = self.inner.lock().unwrap();

        // Sync
        if let Some(sender) = &inner.sender {
            let _ = sender.send(event.clone());
        }

        // Async sinks
        for sink in &inner.async_sinks {
            sink.lock().unwrap().push(event.clone());
        }

        // Buffered async sender
        if let Some(buffered_sender) = &inner.buffered_async_sender {
            let sender = buffered_sender.clone();
            drop(inner); // Release lock before awaiting
            let _ = sender.send_event(event).await;
        }
    }

    fn notify(&self, event: RendererEvent) {
        let inner = self.inner.lock().unwrap();
        // Sync
        if let Some(sender) = &inner.sender {
            let _ = sender.send(event.clone());
        }
        // Async sinks
        for sink in &inner.async_sinks {
            sink.lock().unwrap().push(event.clone());
        }
    }

    pub fn add(&self, kind: RendererKind) {
        let mut inner = self.inner.lock().unwrap();
        inner.renderers.entry(kind).or_insert_with(|| kind.create());
    }

    pub async fn start_async(&self, kind: RendererKind) -> Result<(), String> {
        let result = {
            let mut inner = self.inner.lock().unwrap();
            inner.renderers.entry(kind).or_insert_with(|| kind.create());
            if let Some(renderer) = inner.renderers.get_mut(&kind) {
                renderer.start()?;
                inner.active = Some(kind);
                Ok(())
            } else {
                Err(format!("Renderer {:?} not found", kind))
            }
        };

        if result.is_ok() {
            self.notify_async(RendererEvent::Started(kind)).await;
            self.notify_async(RendererEvent::Switched(Some(kind))).await;
        }

        result
    }

    pub fn start(&self, kind: RendererKind) -> Result<(), String> {
        let mut inner = self.inner.lock().unwrap();
        inner.renderers.entry(kind).or_insert_with(|| kind.create());
        if let Some(renderer) = inner.renderers.get_mut(&kind) {
            renderer.start()?;
            inner.active = Some(kind);
            drop(inner);
            self.notify(RendererEvent::Started(kind));
            self.notify(RendererEvent::Switched(Some(kind)));
            Ok(())
        } else {
            Err(format!("Renderer {:?} not found", kind))
        }
    }

    pub async fn stop_async(&self, kind: RendererKind) {
        let (was_active, stopped) = {
            let mut inner = self.inner.lock().unwrap();
            let stopped = if let Some(renderer) = inner.renderers.get_mut(&kind) {
                renderer.stop();
                true
            } else {
                false
            };
            let was_active = inner.active == Some(kind);
            if was_active {
                inner.active = None;
            }
            (was_active, stopped)
        };

        if stopped {
            self.notify_async(RendererEvent::Stopped(kind)).await;
            if was_active {
                self.notify_async(RendererEvent::Switched(None)).await;
            }
        }
    }

    pub fn stop(&self, kind: RendererKind) {
        let mut inner = self.inner.lock().unwrap();
        if let Some(renderer) = inner.renderers.get_mut(&kind) {
            renderer.stop();
            let was_active = inner.active == Some(kind);
            if was_active {
                inner.active = None;
            }
            drop(inner);
            self.notify(RendererEvent::Stopped(kind));
            if was_active {
                self.notify(RendererEvent::Switched(None));
            }
        }
    }

    pub fn render_frame(&self) -> Result<(), String> {
        let mut inner = self.inner.lock().unwrap();
        match inner.active {
            Some(kind) => {
                if let Some(renderer) = inner.renderers.get_mut(&kind) {
                    renderer.render_frame()
                } else {
                    Err("Active renderer not found".into())
                }
            }
            None => Err("No active renderer".into()),
        }
    }

    pub async fn switch_async(&self, kind: RendererKind) -> Result<(), String> {
        {
            let mut inner = self.inner.lock().unwrap();
            if let Some(active) = inner.active {
                if active == kind {
                    return Ok(()); // already active
                }
                if let Some(renderer) = inner.renderers.get_mut(&active) {
                    renderer.stop();
                }
                inner.active = None;
            }
        }

        self.notify_async(RendererEvent::Stopped(kind)).await;
        self.notify_async(RendererEvent::Switched(None)).await;
        self.start_async(kind).await
    }

    pub fn switch(&self, kind: RendererKind) -> Result<(), String> {
        {
            let mut inner = self.inner.lock().unwrap();
            if let Some(active) = inner.active {
                if active == kind {
                    return Ok(()); // already active
                }
                if let Some(renderer) = inner.renderers.get_mut(&active) {
                    renderer.stop();
                }
                inner.active = None;
                drop(inner);
                self.notify(RendererEvent::Stopped(kind));
                self.notify(RendererEvent::Switched(None));
            }
        }
        self.start(kind)
    }

    pub async fn stop_all_async(&self) {
        let renderer_kinds: Vec<RendererKind> = {
            let mut inner = self.inner.lock().unwrap();
            let kinds: Vec<_> = inner.renderers.keys().cloned().collect();
            for (_, renderer) in inner.renderers.iter_mut() {
                renderer.stop();
            }
            inner.active = None;
            kinds
        };

        for kind in renderer_kinds {
            self.notify_async(RendererEvent::Stopped(kind)).await;
        }
        self.notify_async(RendererEvent::Switched(None)).await;
    }

    pub fn stop_all(&self) {
        let mut inner = self.inner.lock().unwrap();
        for (kind, renderer) in inner.renderers.iter_mut() {
            renderer.stop();
            self.notify(RendererEvent::Stopped(*kind));
        }
        inner.active = None;
        drop(inner);
        self.notify(RendererEvent::Switched(None));
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
    /// use crate::renderer::manager::RendererManager;
    /// use crate::renderer::factory::MockRendererFactory;
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
    /// use crate::renderer::manager::RendererManager;
    /// use crate::renderer::factory::MockRendererFactory;
    /// use crate::renderer::DataPrecision;
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
    /// use crate::renderer::manager::RendererManager;
    /// use crate::renderer::factory::MockRendererFactory;
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
    /// use crate::renderer::manager::RendererManager;
    /// use crate::renderer::DataPrecision;
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
    ) -> Result<Box<dyn Renderer>, RendererError> {
        let factories = self.get_factories_lock()?;

        // Find factory by name
        for (_, factory) in factories.iter() {
            let info = factory.get_info();
            if info.name == name {
                // Found matching factory, create renderer
                return factory.create(precision, parameters);
            }
        }

        // No factory found with the specified name
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
    /// use crate::renderer::manager::RendererManager;
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
    /// use crate::renderer::manager::RendererManager;
    /// use crate::renderer::DataPrecision;
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
    /// use crate::renderer::manager::RendererManager;
    ///
    /// let manager = RendererManager::new();
    /// println!("Registered factories: {}", manager.get_factory_count());
    /// ```
    pub fn get_factory_count(&self) -> usize {
        match self.get_factories_lock() {
            Ok(factories) => factories.len(),
            Err(_) => 0,
        }
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
    /// use crate::renderer::manager::RendererManager;
    /// use crate::renderer::DataPrecision;
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

        // Find factory by name
        for factory in factories.values() {
            let info = factory.get_info();
            if info.name == factory_name {
                // Found matching factory, validate parameters
                return factory.validate_parameters(precision, parameters);
            }
        }

        // No factory found with the specified name
        Err(RendererError::RendererNotFoundByName(factory_name.to_string()))
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
    /// use crate::renderer::manager::RendererManager;
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
    /// use crate::renderer::manager::RendererManager;
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
    /// use crate::renderer::manager::RendererManager;
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

// Ensure RendererManager is Send + Sync for multi-threaded use
unsafe impl Send for RendererManager {}
unsafe impl Sync for RendererManager {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::renderer::factory::{MockRendererFactory};
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
    fn test_create_by_name_success() {
        let manager = create_test_manager_with_factories();

        // Test creating by existing names
        let cpu_renderer = manager.create_by_name("CpuRenderer", DataPrecision::F32, "test");
        assert!(cpu_renderer.is_ok());

        let gpu_renderer = manager.create_by_name("GpuRenderer", DataPrecision::F16, "test");
        assert!(gpu_renderer.is_ok());

        let precision_renderer = manager.create_by_name("PrecisionRenderer", DataPrecision::F64, "test");
        assert!(precision_renderer.is_ok());
    }

    #[test]
    fn test_create_by_name_not_found() {
        let manager = create_test_manager_with_factories();

        // Test creating with non-existent name
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
        let manager = create_test_manager_with_factories();

        // Try to create CpuRenderer with unsupported BFloat16
        let result = manager.create_by_name("CpuRenderer", DataPrecision::BFloat16, "test");
        assert!(result.is_err());

        match result.unwrap_err() {
            RendererError::UnsupportedPrecision(DataPrecision::BFloat16) => {},
            _ => panic!("Expected UnsupportedPrecision error"),
        }
    }

    #[test]
    fn test_find_by_capability_success() {
        let manager = create_test_manager_with_factories();

        // Find renderers with cpu_rendering capability
        let cpu_renderers = manager.find_by_capability("cpu_rendering");
        assert_eq!(cpu_renderers.len(), 2); // CpuRenderer and PrecisionRenderer

        let names: Vec<&str> = cpu_renderers.iter().map(|info| info.name.as_str()).collect();
        assert!(names.contains(&"CpuRenderer"));
        assert!(names.contains(&"PrecisionRenderer"));

        // Find renderers with gpu_rendering capability
        let gpu_renderers = manager.find_by_capability("gpu_rendering");
        assert_eq!(gpu_renderers.len(), 1);
        assert_eq!(gpu_renderers[0].name, "GpuRenderer");

        // Find renderers with real_time capability
        let realtime_renderers = manager.find_by_capability("real_time");
        assert_eq!(realtime_renderers.len(), 1);
        assert_eq!(realtime_renderers[0].name, "GpuRenderer");
    }

    #[test]
    fn test_find_by_capability_not_found() {
        let manager = create_test_manager_with_factories();

        // Search for non-existent capability
        let result = manager.find_by_capability("quantum_rendering");
        assert!(result.is_empty());

        // Search for capability with different case
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
        let manager = create_test_manager_with_factories();

        // Test that capability matching handles whitespace correctly
        let result = manager.find_by_capability(" cpu_rendering ");
        assert!(result.is_empty()); // Should not match due to leading/trailing spaces

        let result = manager.find_by_capability("cpu_rendering");
        assert_eq!(result.len(), 2); // Should match exactly
    }

    #[test]
    fn test_find_by_precision_success() {
        let manager = create_test_manager_with_factories();

        // Find renderers supporting F32
        let f32_renderers = manager.find_by_precision(DataPrecision::F32);
        assert_eq!(f32_renderers.len(), 2); // CpuRenderer and GpuRenderer

        // Find renderers supporting F64
        let f64_renderers = manager.find_by_precision(DataPrecision::F64);
        assert_eq!(f64_renderers.len(), 2); // CpuRenderer and PrecisionRenderer

        // Find renderers supporting F16
        let f16_renderers = manager.find_by_precision(DataPrecision::F16);
        assert_eq!(f16_renderers.len(), 1); // Only GpuRenderer
        assert_eq!(f16_renderers[0].name, "GpuRenderer");

        // Find renderers supporting BFloat16
        let bf16_renderers = manager.find_by_precision(DataPrecision::BFloat16);
        assert_eq!(bf16_renderers.len(), 1); // Only GpuRenderer
        assert_eq!(bf16_renderers[0].name, "GpuRenderer");
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
        let manager = create_test_manager_with_factories();

        // Test valid parameters for existing factories
        let result = manager.validate_parameters_for("CpuRenderer", DataPrecision::F32, "test");
        assert!(result.is_ok());

        let result = manager.validate_parameters_for("GpuRenderer", DataPrecision::F16, "custom_name=true");
        assert!(result.is_ok());

        let result = manager.validate_parameters_for("PrecisionRenderer", DataPrecision::F64, "test_mode=debug");
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
        let manager = create_test_manager_with_factories();

        let cpu_info = manager.find_factory_by_name("CpuRenderer");
        assert!(cpu_info.is_some());
        let cpu_info = cpu_info.unwrap();
        assert_eq!(cpu_info.name, "CpuRenderer");
        assert!(cpu_info.has_capability("cpu_rendering"));

        let gpu_info = manager.find_factory_by_name("GpuRenderer");
        assert!(gpu_info.is_some());
        let gpu_info = gpu_info.unwrap();
        assert_eq!(gpu_info.name, "GpuRenderer");
        assert!(gpu_info.has_capability("gpu_rendering"));
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
        let manager = create_test_manager_with_factories();

        let capabilities = manager.get_all_capabilities();

        // Should contain all unique capabilities from all factories
        let expected_capabilities = vec![
            "advanced_3d",
            "basic_3d",
            "cpu_rendering",
            "gpu_rendering",
            "hardware_accelerated",
            "high_precision",
            "real_time",
            "scientific",
            "software",
        ];

        assert_eq!(capabilities.len(), expected_capabilities.len());
        for expected_cap in &expected_capabilities {
            assert!(capabilities.contains(&expected_cap.to_string()),
                    "Missing capability: {}", expected_cap);
        }

        // Verify capabilities are sorted
        let mut sorted_capabilities = capabilities.clone();
        sorted_capabilities.sort();
        assert_eq!(capabilities, sorted_capabilities);
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
        let manager = Arc::new(create_test_manager_with_factories());
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
                        let _info = manager_clone.find_factory_by_name("CpuRenderer");
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
        let manager = create_test_manager_with_factories();

        // Test that parameter validation errors are properly propagated
        let result = manager.validate_parameters_for("CpuRenderer", DataPrecision::F32, "invalid_params");
        assert!(result.is_err());

        // Test that creation errors are properly propagated through create_by_name
        let result = manager.create_by_name("CpuRenderer", DataPrecision::F32, "invalid_params");
        assert!(result.is_err());

        match result.unwrap_err() {
            RendererError::InvalidParameters(_) => {},
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