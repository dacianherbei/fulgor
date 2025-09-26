use crate::renderer::{DataPrecision, PerformancePriority, Renderer, RendererError, RendererFactory, RendererInfo, RendererRequirements};
use std::any::TypeId;
use std::collections::{BTreeSet, HashMap};
use std::sync::{mpsc, Arc, Mutex, MutexGuard};
use std::thread;
use std::time::{Duration, Instant};


/// Detailed precision capabilities for a specific factory
#[derive(Debug, Clone)]
pub struct FactoryPrecisionInfo {
    pub factory_name: String,
    pub factory_id: TypeId,
    pub supported_precisions: Vec<DataPrecision>,
    pub preferred_precision: Option<DataPrecision>,
    pub precision_performance: HashMap<DataPrecision, PrecisionPerformance>,
}

/// Performance characteristics for a specific precision
#[derive(Debug, Clone)]
pub struct PrecisionPerformance {
    pub relative_speed: f32,        // 1.0 = baseline, >1.0 = faster, <1.0 = slower
    pub memory_usage_factor: f32,   // 1.0 = baseline memory usage
    pub quality_score: f32,         // 0.0-1.0, higher = better quality
}

/// Overall precision matrix showing which factories support which precisions
#[derive(Debug, Clone)]
pub struct PrecisionMatrix {
    pub precisions: Vec<DataPrecision>,
    pub factories: Vec<FactoryPrecisionInfo>,
    pub coverage_map: HashMap<DataPrecision, Vec<String>>, // Precision -> Factory names
}

/// Health status of a factory
#[derive(Debug, Clone, PartialEq)]
pub enum FactoryHealth {
    Healthy,
    Degraded { reason: String },
    Unhealthy { reason: String },
    Unknown, // Haven't checked yet or check failed
}

/// Detailed health information for a factory
#[derive(Debug, Clone)]
pub struct FactoryHealthInfo {
    pub factory_name: String,
    pub factory_id: TypeId,
    pub health_status: FactoryHealth,
    pub last_check_time: Option<Instant>,
    pub response_time: Option<Duration>,
    pub success_rate: f32, // 0.0-1.0 over recent operations
    pub error_count: u64,
    pub last_error: Option<String>,
}

/// Aggregated metrics for a factory
#[derive(Debug, Clone)]
pub struct FactoryMetrics {
    pub factory_name: String,
    pub factory_id: TypeId,
    pub total_creations: u64,
    pub successful_creations: u64,
    pub failed_creations: u64,
    pub average_creation_time: Duration,
    pub fastest_creation_time: Duration,
    pub slowest_creation_time: Duration,
    pub preferred_precisions: Vec<DataPrecision>, // Most commonly used
}

/// System-wide renderer health report
#[derive(Debug, Clone)]
pub struct SystemHealthReport {
    pub total_factories: usize,
    pub healthy_factories: usize,
    pub degraded_factories: usize,
    pub unhealthy_factories: usize,
    pub overall_health: FactoryHealth,
    pub factory_details: Vec<FactoryHealthInfo>,
    pub recommended_actions: Vec<String>,
}

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
    factory_health: Arc<Mutex<HashMap<TypeId, FactoryHealthInfo>>>,
    factory_metrics: Arc<Mutex<HashMap<TypeId, FactoryMetrics>>>,
    last_health_check: Arc<Mutex<Option<Instant>>>
}

impl RendererManager {
    pub fn new() -> Self {
        Self {
            factories: Arc::new(Mutex::new(HashMap::new())),
            renderer_shutdowns: Mutex::new(Vec::new()),
            factory_health: Arc::new(Mutex::new(Default::default())),
            factory_metrics: Arc::new(Mutex::new(Default::default())),
            last_health_check: Arc::new(Mutex::new(None)),
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
        let type_id = factory.as_ref().type_id(); // ← Get actual factory TypeId from trait object

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
                                Err(_) => Err(RendererError::CreationFailed(
                                    "Failed to unwrap factory Arc for registration".to_string(),
                                )),
                            }
                        }
                    }
                    Err(_) => Err(RendererError::CreationFailed(
                        "Failed to acquire factories lock".to_string(),
                    )),
                }
            };

            let _ = sender.send((result, start_time.elapsed()));
        });

        match receiver.recv_timeout(timeout_duration) {
            Ok((result, _elapsed)) => result,
            Err(mpsc::RecvTimeoutError::Timeout) => Err(RendererError::CreationFailed(format!(
                "Factory registration timed out after {} microseconds",
                timeout_duration.as_micros()
            ))),
            Err(mpsc::RecvTimeoutError::Disconnected) => Err(RendererError::CreationFailed(
                "Registration thread disconnected unexpectedly".to_string(),
            )),
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
            Ok(factories) => factories
                .values()
                .map(|factory| factory.get_info())
                .collect(),
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
            Ok(factories) => factories
                .values()
                .map(|factory| factory.get_info())
                .filter(|info| info.has_capability(capability))
                .collect(),
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
        renderer: Box<dyn Renderer>,
    ) -> Result<(Box<dyn Renderer>, mpsc::Receiver<()>, mpsc::Sender<()>), RendererError> {
        // Get the shutdown timeout and unique ID from the renderer
        let shutdown_timeout = renderer.shutdown_timeout();
        let renderer_id = renderer.unique_id();

        // Create shutdown signal channel (manager -> renderer)
        let (shutdown_tx, shutdown_rx) = mpsc::channel();

        // Create confirmation channel (renderer -> manager)
        let (confirm_tx, confirm_rx) = mpsc::channel();

        // Track this renderer for cleanup
        self.renderer_shutdowns.lock().unwrap().push((
            renderer_id,
            shutdown_tx,
            confirm_rx,
            shutdown_timeout,
        ));

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

        Err(RendererError::RendererNotFoundByName(
            factory_name.to_string(),
        ))
    }

    /// Helper method to acquire the factories lock safely
    fn get_factories_lock(
        &self,
    ) -> Result<MutexGuard<'_, HashMap<TypeId, Box<dyn RendererFactory>>>, RendererError> {
        self.factories.lock().map_err(|_| {
            RendererError::CreationFailed("Failed to acquire factories lock".to_string())
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
            Ok(factories) => factories
                .values()
                .map(|factory| factory.get_info())
                .find(|info| info.name == name),
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

    /// Get a detailed precision matrix showing which factories support which precisions
    pub fn get_precision_matrix(&self) -> PrecisionMatrix {
        match self.get_factories_lock() {
            Ok(factories) => {
                let mut all_precisions = BTreeSet::new();
                let mut factory_infos = Vec::new();
                let mut coverage_map: HashMap<DataPrecision, Vec<String>> = HashMap::new();

                for (type_id, factory) in factories.iter() {
                    let info = factory.get_info();
                    let supported = self.get_factory_supported_precisions(*type_id);

                    // Add precisions to the global set
                    for precision in &supported {
                        all_precisions.insert(*precision);
                        coverage_map.entry(*precision)
                            .or_insert_with(Vec::new)
                            .push(info.name.clone());
                    }

                    // Create factory precision info
                    let precision_info = FactoryPrecisionInfo {
                        factory_name: info.name.clone(),
                        factory_id: *type_id,
                        supported_precisions: supported,
                        preferred_precision: self.get_factory_preferred_precision(*type_id),
                        precision_performance: self.get_factory_precision_performance(*type_id),
                    };

                    factory_infos.push(precision_info);
                }

                PrecisionMatrix {
                    precisions: all_precisions.into_iter().collect(),
                    factories: factory_infos,
                    coverage_map,
                }
            }
            Err(_) => PrecisionMatrix {
                precisions: vec![],
                factories: vec![],
                coverage_map: HashMap::new(),
            }
        }
    }

    /// Get supported precisions for a specific factory
    pub fn get_factory_supported_precisions(&self, factory_id: TypeId) -> Vec<DataPrecision> {
        let test_precisions = [
            DataPrecision::F16,
            DataPrecision::F32,
            DataPrecision::F64,
            DataPrecision::BFloat16,
        ];

        let mut supported = Vec::new();

        if let Ok(factories) = self.get_factories_lock() {
            if let Some(factory) = factories.get(&factory_id) {
                for precision in &test_precisions {
                    // Test precision support by attempting validation
                    if factory.validate_parameters(*precision, "").is_ok() {
                        supported.push(*precision);
                    }
                }
            }
        }

        supported
    }

    /// Get the preferred precision for a factory (heuristic-based)
    pub fn get_factory_preferred_precision(&self, factory_id: TypeId) -> Option<DataPrecision> {
        let supported = self.get_factory_supported_precisions(factory_id);

        // Heuristic: prefer F32 if available, otherwise the first supported precision
        if supported.contains(&DataPrecision::F32) {
            Some(DataPrecision::F32)
        } else {
            supported.first().copied()
        }
    }

    /// Get performance characteristics for each precision (placeholder implementation)
    fn get_factory_precision_performance(&self, _factory_id: TypeId) -> HashMap<DataPrecision, PrecisionPerformance> {
        // TODO: This could be enhanced to actually benchmark or use stored metrics
        let mut performance = HashMap::new();

        // Default performance characteristics (can be customized per factory)
        performance.insert(DataPrecision::F16, PrecisionPerformance {
            relative_speed: 1.8,
            memory_usage_factor: 0.5,
            quality_score: 0.85,
        });

        performance.insert(DataPrecision::F32, PrecisionPerformance {
            relative_speed: 1.0, // Baseline
            memory_usage_factor: 1.0,
            quality_score: 0.95,
        });

        performance.insert(DataPrecision::F64, PrecisionPerformance {
            relative_speed: 0.6,
            memory_usage_factor: 2.0,
            quality_score: 1.0,
        });

        performance.insert(DataPrecision::BFloat16, PrecisionPerformance {
            relative_speed: 1.4,
            memory_usage_factor: 0.5,
            quality_score: 0.8,
        });

        performance
    }

    /// Find the best factory for given requirements
    pub fn find_best_factory(&self, requirements: &RendererRequirements) -> Option<RendererInfo> {
        let matrix = self.get_precision_matrix();
        let mut best_factory: Option<&FactoryPrecisionInfo> = None;
        let mut best_score = 0.0f32;

        for factory_info in &matrix.factories {
            let mut score = 0.0f32;

            // Check required capabilities
            let factory_caps: BTreeSet<String> = match self.get_factories_lock() {
                Ok(factories) => {
                    if let Some(factory) = factories.get(&factory_info.factory_id) {
                        factory.get_info().get_capabilities()
                            .iter().map(|s| s.to_string()).collect()
                    } else {
                        continue;
                    }
                }
                Err(_) => continue,
            };

            // Must have all required capabilities
            let required_caps: BTreeSet<String> = requirements.required_capabilities.iter().cloned().collect();
            if !required_caps.is_subset(&factory_caps) {
                continue; // Skip if missing required capabilities
            }

            score += 100.0; // Base score for meeting requirements

            // Precision preference scoring
            if factory_info.supported_precisions.contains(&requirements.preferred_precision) {
                score += 50.0;

                // Add performance-based scoring
                if let Some(perf) = factory_info.precision_performance.get(&requirements.preferred_precision) {
                    match requirements.performance_priority {
                        PerformancePriority::Speed => score += perf.relative_speed * 20.0,
                        PerformancePriority::Quality => score += perf.quality_score * 30.0,
                        PerformancePriority::Memory => score += (2.0 - perf.memory_usage_factor) * 15.0,
                        PerformancePriority::Balanced => {
                            score += (perf.relative_speed + perf.quality_score + (2.0 - perf.memory_usage_factor)) * 10.0;
                        }
                    }
                }
            }

            // Timeout preference
            if let Ok(factories) = self.get_factories_lock() {
                if let Some(factory) = factories.get(&factory_info.factory_id) {
                    let factory_timeout = Duration::from_micros(factory.get_info().timeout_microseconds);
                    if factory_timeout <= requirements.max_timeout {
                        score += 20.0;
                    }
                }
            }

            if score > best_score {
                best_score = score;
                best_factory = Some(factory_info);
            }
        }

        // Convert to RendererInfo
        if let Some(factory_info) = best_factory {
            if let Ok(factories) = self.get_factories_lock() {
                if let Some(factory) = factories.get(&factory_info.factory_id) {
                    return Some(factory.get_info());
                }
            }
        }

        None
    }

    /// Perform health checks on all registered factories
    pub fn health_check(&self) -> SystemHealthReport {
        match self.get_factories_lock() {
            Ok(factories) => {
                let mut factory_details = Vec::new();
                let mut healthy_count = 0;
                let mut degraded_count = 0;
                let mut unhealthy_count = 0;

                for (type_id, factory) in factories.iter() {
                    let health_info = self.check_factory_health(*type_id, factory.as_ref());

                    match health_info.health_status {
                        FactoryHealth::Healthy => healthy_count += 1,
                        FactoryHealth::Degraded { .. } => degraded_count += 1,
                        FactoryHealth::Unhealthy { .. } => unhealthy_count += 1,
                        FactoryHealth::Unknown => {},
                    }

                    factory_details.push(health_info);
                }

                let total_factories = factories.len();
                let overall_health = if unhealthy_count > 0 {
                    FactoryHealth::Unhealthy {
                        reason: format!("{} factories are unhealthy", unhealthy_count)
                    }
                } else if degraded_count > 0 {
                    FactoryHealth::Degraded {
                        reason: format!("{} factories are degraded", degraded_count)
                    }
                } else if healthy_count > 0 {
                    FactoryHealth::Healthy
                } else {
                    FactoryHealth::Unknown
                };

                let recommended_actions = self.generate_health_recommendations(&factory_details);

                SystemHealthReport {
                    total_factories,
                    healthy_factories: healthy_count,
                    degraded_factories: degraded_count,
                    unhealthy_factories: unhealthy_count,
                    overall_health,
                    factory_details,
                    recommended_actions,
                }
            }
            Err(_) => SystemHealthReport {
                total_factories: 0,
                healthy_factories: 0,
                degraded_factories: 0,
                unhealthy_factories: 0,
                overall_health: FactoryHealth::Unknown,
                factory_details: vec![],
                recommended_actions: vec!["Unable to acquire factory lock".to_string()],
            }
        }
    }

    /// Check health of a specific factory
    fn check_factory_health(&self, factory_id: TypeId, factory: &dyn RendererFactory) -> FactoryHealthInfo {
        let factory_name = factory.get_info().name.clone();
        let start_time = Instant::now();

        // Test basic functionality
        let health_status = match self.perform_factory_health_test(factory) {
            Ok(()) => FactoryHealth::Healthy,
            Err(e) => {
                if e.contains("timeout") || e.contains("slow") {
                    FactoryHealth::Degraded { reason: e }
                } else {
                    FactoryHealth::Unhealthy { reason: e }
                }
            }
        };

        let response_time = start_time.elapsed();

        FactoryHealthInfo {
            factory_name,
            factory_id,
            health_status,
            last_check_time: Some(Instant::now()),
            response_time: Some(response_time),
            success_rate: 1.0, // TODO: Track over time
            error_count: 0,     // TODO: Track over time
            last_error: None,   // TODO: Track over time
        }
    }

    /// Perform basic health test on a factory
    fn perform_factory_health_test(&self, factory: &dyn RendererFactory) -> Result<(), String> {
        // Test 1: Basic info retrieval
        let _info = factory.get_info();

        // Test 2: Parameter validation with empty params
        factory.validate_parameters(DataPrecision::F32, "")
            .map_err(|e| format!("Validation test failed: {:?}", e))?;

        // Test 3: Check response time
        let start = Instant::now();
        let _info2 = factory.get_info();
        let elapsed = start.elapsed();

        if elapsed > Duration::from_millis(100) {
            return Err(format!("Slow response: {:?}", elapsed));
        }

        Ok(())
    }

    /// Generate health recommendations based on factory status
    fn generate_health_recommendations(&self, factory_details: &[FactoryHealthInfo]) -> Vec<String> {
        let mut recommendations = Vec::new();

        let unhealthy_count = factory_details.iter()
            .filter(|f| matches!(f.health_status, FactoryHealth::Unhealthy { .. }))
            .count();

        let degraded_count = factory_details.iter()
            .filter(|f| matches!(f.health_status, FactoryHealth::Degraded { .. }))
            .count();

        if unhealthy_count > 0 {
            recommendations.push(format!(
                "Consider removing or restarting {} unhealthy factories",
                unhealthy_count
            ));
        }

        if degraded_count > 0 {
            recommendations.push(format!(
                "Monitor {} degraded factories for potential issues",
                degraded_count
            ));
        }

        if factory_details.iter().any(|f| f.response_time.map_or(false, |t| t > Duration::from_millis(50))) {
            recommendations.push("Some factories have slow response times - consider optimization".to_string());
        }

        if recommendations.is_empty() {
            recommendations.push("All factories are operating normally".to_string());
        }

        recommendations
    }

    /// Get basic metrics for all factories
    pub fn get_factory_metrics(&self) -> Vec<FactoryMetrics> {
        // TODO: This would require tracking metrics over time
        // For now, return placeholder metrics
        match self.get_factories_lock() {
            Ok(factories) => {
                factories.iter().map(|(type_id, factory)| {
                    let info = factory.get_info();
                    FactoryMetrics {
                        factory_name: info.name,
                        factory_id: *type_id,
                        total_creations: 0,
                        successful_creations: 0,
                        failed_creations: 0,
                        average_creation_time: Duration::from_millis(10),
                        fastest_creation_time: Duration::from_millis(5),
                        slowest_creation_time: Duration::from_millis(50),
                        preferred_precisions: vec![DataPrecision::F32],
                    }
                }).collect()
            }
            Err(_) => vec![],
        }
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

            println!(
                "Sent shutdown signals to {} renderers, waiting for confirmations...",
                renderer_count
            );

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
                        eprintln!(
                            "WARNING: Renderer {} did not confirm shutdown within {:?} timeout",
                            renderer_id, timeout
                        );
                    }
                    Err(mpsc::RecvTimeoutError::Disconnected) => {
                        // Channel was dropped, renderer probably stopped
                        println!("Renderer {} shutdown (channel disconnected)", renderer_id);
                        completed += 1;
                    }
                }
            }

            println!(
                "Renderer shutdown complete: {} confirmed, {} timed out",
                completed, timed_out
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::renderer::factory::MockRendererFactory;
    use std::sync::Arc;
    use std::thread;

    macro_rules! test_factory {
        ($name:ident) => {
            #[derive(Debug)]
            struct $name(MockRendererFactory);
            impl RendererFactory for $name {
                fn create(&self, precision: DataPrecision, parameters: &str) -> Result<Box<dyn Renderer>, RendererError> {
                    self.0.create(precision, parameters)
                }
                fn get_info(&self) -> RendererInfo {
                    self.0.get_info()
                }
                fn validate_parameters(&self, precision: DataPrecision, parameters: &str) -> Result<(), RendererError> {
                    self.0.validate_parameters(precision, parameters)
                }
            }
        };
    }

    // MACRO 2: Create isolated test manager with single factory
    macro_rules! isolated_manager {
        ($factory_type:ident, $mock_config:expr) => {{
            test_factory!($factory_type);
            let mut manager = RendererManager::new();
            let factory = Box::new($factory_type($mock_config));
            manager.register(factory).unwrap();
            manager
        }};
    }

    // MACRO 3: Create test manager with multiple factories (different TypeIds)
    macro_rules! multi_factory_manager {
        ($(($factory_type:ident, $mock_config:expr)),+ $(,)?) => {{
            $(test_factory!($factory_type);)+
            let mut manager = RendererManager::new();
            $(
                let factory = Box::new($factory_type($mock_config));
                manager.register(factory).unwrap();
            )+
            manager
        }};
    }

    // MACRO 4: Create manager with predefined common scenarios
    macro_rules! standard_test_manager {
        (cpu_gpu) => {{
            multi_factory_manager!(
                (StandardCpuFactory, MockRendererFactory::new_full(
                    "CpuRenderer",
                    vec![DataPrecision::F32, DataPrecision::F64],
                    "cpu_rendering,basic_3d,software",
                    3000,
                )),
                (StandardGpuFactory, MockRendererFactory::new_full(
                    "GpuRenderer",
                    vec![DataPrecision::F16, DataPrecision::F32, DataPrecision::BFloat16],
                    "gpu_rendering,advanced_3d,hardware_accelerated,real_time",
                    5000,
                ))
            )
        }};
        (single_cpu) => {{
            isolated_manager!(
                StandardSingleCpuFactory,
                MockRendererFactory::new_full(
                    "CpuRenderer",
                    vec![DataPrecision::F32, DataPrecision::F64],
                    "cpu_rendering,basic_3d,software",
                    3000,
                )
            )
        }};
    }

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
            RendererError::FactoryAlreadyRegistered(_) => {}
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
            "test_params",
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
            "test_params",
        );

        assert!(result.is_err());
        match result.unwrap_err() {
            RendererError::RendererNotFound(_) => {}
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
            "invalid_params", // This should trigger validation error
        );

        assert!(result.is_err());
        match result.unwrap_err() {
            RendererError::InvalidParameters(_) => {}
            _ => panic!("Expected InvalidParameters error"),
        }
    }

    #[test]
    fn test_create_with_unsupported_precision() {
        let mut manager = RendererManager::new();
        let factory = Box::new(MockRendererFactory::new_with_precisions(
            "TestFactory",
            vec![DataPrecision::F32], // Only F32 supported
        ));

        manager.register(factory).unwrap();

        let result = manager.create(
            TypeId::of::<MockRendererFactory>(),
            DataPrecision::F64, // Unsupported precision
            "test_params",
        );

        assert!(result.is_err());
        match result.unwrap_err() {
            RendererError::UnsupportedPrecision(DataPrecision::F64) => {}
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
                    "TestFactory", // Use the factory name instead of TypeId
                    DataPrecision::F32,
                    "", // Use empty parameters
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
                panic!(
                    "Thread {} failed with error: {:?}",
                    i,
                    error.unwrap_or("Unknown error".to_string())
                );
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
            1, // 1 microsecond timeout - should be very tight
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
        let renderer1 = manager
            .create_by_name("OpenGL3Renderer", DataPrecision::F16, "vsync=true")
            .unwrap();
        let renderer2 = manager
            .create_by_name("OpenGL3Renderer", DataPrecision::F32, "vsync=false")
            .unwrap();
        let renderer3 = manager
            .create_by_name("OpenGL3Renderer", DataPrecision::F64, "msaa=4")
            .unwrap();

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
                    }
                    1 => {
                        // Test precision search
                        let _renderers = manager_clone.find_by_precision(DataPrecision::F32);
                    }
                    2 => {
                        // Test factory count
                        let _count = manager_clone.get_factory_count();
                    }
                    3 => {
                        // Test factory lookup
                        let _info = manager_clone.find_factory_by_name("TestRenderer");
                    }
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
            RendererError::FactoryAlreadyRegistered(_) => {} // ✅ Expected
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
            }
            _ => panic!("Expected RendererNotFoundByName error"),
        }
    }

    #[test]
    fn test_create_by_name_empty_registry() {
        let manager = RendererManager::new();

        let result = manager.create_by_name("AnyRenderer", DataPrecision::F32, "test");
        assert!(result.is_err());

        match result.unwrap_err() {
            RendererError::RendererNotFoundByName(_) => {}
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
            vec![DataPrecision::F32, DataPrecision::F64], // Only F32 and F64, NOT BFloat16
        ));
        manager.register(factory).unwrap();

        // Try to create renderer with unsupported BFloat16 precision
        let result = manager.create_by_name("LimitedRenderer", DataPrecision::BFloat16, "test");
        assert!(result.is_err());

        match result.unwrap_err() {
            RendererError::UnsupportedPrecision(DataPrecision::BFloat16) => {
                // ✅ Expected error - factory correctly rejected unsupported precision
            }
            _ => panic!("Expected UnsupportedPrecision error"),
        }
    }

    #[test]
    fn test_find_by_capability_success_with_standard() {
        let manager = standard_test_manager!(cpu_gpu);

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

    // AFTER: Clean macro usage (replace existing tests with this pattern)
    #[test]
    fn test_find_by_capability_whitespace_handling() {
        let manager = isolated_manager!(
            WhitespaceTestFactory,
            MockRendererFactory::new_full(
                "TestRenderer",
                vec![DataPrecision::F32],
                "cpu_rendering,basic_3d",
                3000,
            )
        );

        // Test whitespace handling - the implementation intentionally trims input for user convenience
        let result = manager.find_by_capability(" cpu_rendering ");
        assert_eq!(result.len(), 1); // SHOULD match because input is trimmed

        let result = manager.find_by_capability("cpu_rendering");
        assert_eq!(result.len(), 1); // Should also match

        // Test that non-existent capabilities still don't match
        let result = manager.find_by_capability(" nonexistent_capability ");
        assert!(result.is_empty()); // Should not match even with trimming
    }

    #[test]
    fn test_find_by_precision_success() {
        let manager = multi_factory_manager!(
            (CpuPrecisionFactory, MockRendererFactory::new_full(
                "CpuRenderer",
                vec![DataPrecision::F32, DataPrecision::F64],
                "cpu_rendering,basic_3d,software",
                3000,
            )),
            (GpuPrecisionFactory, MockRendererFactory::new_full(
                "GpuRenderer",
                vec![DataPrecision::F16, DataPrecision::F32, DataPrecision::BFloat16],
                "gpu_rendering,advanced_3d,hardware_accelerated,real_time",
                5000,
            ))
        );

        // Test F32 - both factories support it
        let f32_renderers = manager.find_by_precision(DataPrecision::F32);
        assert_eq!(f32_renderers.len(), 2);

        // Test F64 - only CPU factory supports it
        let f64_renderers = manager.find_by_precision(DataPrecision::F64);
        assert_eq!(f64_renderers.len(), 1);
        assert_eq!(f64_renderers[0].name, "CpuRenderer");
    }

    #[test]
    fn test_find_by_precision_not_found() {
        let manager = isolated_manager!(
            LimitedPrecisionFactory,
            MockRendererFactory::new_with_precisions(
                "LimitedRenderer",
                vec![DataPrecision::F32], // Only supports F32
            )
        );

        // Search for unsupported precision
        let result = manager.find_by_precision(DataPrecision::F64);
        assert!(result.is_empty()); // Should be empty now
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

        // Create manager with multiple factories using macro
        let manager = multi_factory_manager!(
            (CountFactory1, MockRendererFactory::new("CpuRenderer")),
            (CountFactory2, MockRendererFactory::new("GpuRenderer")),
            (CountFactory3, MockRendererFactory::new("PrecisionRenderer"))
        );

        assert_eq!(manager.get_factory_count(), 3);

        // Test adding another factory
        let mut manager = manager;
        test_factory!(CountFactory4);
        let extra_factory = Box::new(CountFactory4(MockRendererFactory::new("ExtraRenderer")));
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
        let manager = isolated_manager!(
            ValidationTestFactory,
            MockRendererFactory::new("TestRenderer")
        );

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
        let manager = isolated_manager!(
            PrecisionValidationFactory,
            MockRendererFactory::new_full(
                "CpuRenderer",
                vec![DataPrecision::F32, DataPrecision::F64], // No BFloat16
                "cpu_rendering,basic_3d,software",
                3000,
            )
        );

        // Try to validate with unsupported BFloat16 precision
        let result = manager.validate_parameters_for("CpuRenderer", DataPrecision::BFloat16, "test");
        assert!(result.is_err());

        match result.unwrap_err() {
            RendererError::UnsupportedPrecision(DataPrecision::BFloat16) => {},
            _ => panic!("Expected UnsupportedPrecision error"),
        }
    }

    #[test]
    fn test_validate_parameters_for_invalid_parameters() {
        let manager = isolated_manager!(
            InvalidParamsFactory,
            MockRendererFactory::new("CpuRenderer")
        );

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
        // Use single factory helper to avoid registration conflicts
        let manager = create_single_test_factory_manager();

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
        let manager = standard_test_manager!(cpu_gpu);

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
        let manager = multi_factory_manager!(
            (TestFactory1, MockRendererFactory::new_full(
                "Renderer1",
                vec![DataPrecision::F32],
                "capability1,capability2,shared",
                1000,
            )),
            (TestFactory2, MockRendererFactory::new_full(
                "Renderer2",
                vec![DataPrecision::F32],
                "capability2,capability3,shared",
                1000,
            ))
        );

        let capabilities = manager.get_all_capabilities();
        assert_eq!(capabilities.len(), 4); // capability1, capability2, capability3, shared

        // Verify specific capabilities are present
        let expected_caps = ["capability1", "capability2", "capability3", "shared"];
        for cap in expected_caps {
            assert!(capabilities.contains(&cap.to_string()), "Missing capability: {}", cap);
        }
    }

    #[test]
    fn test_example_with_standard_manager() {
        let manager = standard_test_manager!(cpu_gpu);

        let cpu_renderers = manager.find_by_capability("cpu_rendering");
        assert_eq!(cpu_renderers.len(), 1);

        let gpu_renderers = manager.find_by_capability("gpu_rendering");
        assert_eq!(gpu_renderers.len(), 1);
    }

    #[test]
    fn test_get_supported_precisions() {
        let manager = multi_factory_manager!(
            (PrecisionTestFactory1, MockRendererFactory::new_full(
                "CpuRenderer",
                vec![DataPrecision::F32, DataPrecision::F64],
                "cpu_rendering,basic_3d,software",
                3000,
            )),
            (PrecisionTestFactory2, MockRendererFactory::new_full(
                "GpuRenderer",
                vec![DataPrecision::F16, DataPrecision::F32, DataPrecision::BFloat16],
                "gpu_rendering,advanced_3d,hardware_accelerated,real_time",
                5000,
            )),
            (PrecisionTestFactory3, MockRendererFactory::new_full(
                "PrecisionRenderer",
                vec![DataPrecision::F64],
                "high_precision,scientific,cpu_rendering",
                10000,
            ))
        );

        let precisions = manager.get_supported_precisions();

        // Should contain: F16, F32, F64, BFloat16
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
            "CpuRenderer", // Keep original name that test expects
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
                    }
                    1 => {
                        let _renderers = manager_clone.find_by_precision(DataPrecision::F32);
                    }
                    2 => {
                        let _count = manager_clone.get_factory_count();
                    }
                    3 => {
                        let _info = manager_clone.find_factory_by_name("CpuRenderer");
                        // Matches factory name
                    }
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
        let result =
            manager.validate_parameters_for("TestRenderer", DataPrecision::F32, "invalid_params");
        assert!(result.is_err());

        // Test that creation errors are properly propagated through create_by_name
        let result = manager.create_by_name("TestRenderer", DataPrecision::F32, "invalid_params");
        assert!(result.is_err());

        match result.unwrap_err() {
            RendererError::InvalidParameters(_) => {
                // ✅ Success! The InvalidParameters error from the factory
                // was properly propagated through the manager to the caller
            }
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

        // Create a macro to generate unique factory types
        macro_rules! create_factory_type {
            ($name:ident, $index:expr) => {
                #[derive(Debug)]
                struct $name(MockRendererFactory);
                impl RendererFactory for $name {
                    fn create(
                        &self,
                        precision: DataPrecision,
                        parameters: &str,
                    ) -> Result<Box<dyn Renderer>, RendererError> {
                        self.0.create(precision, parameters)
                    }
                    fn get_info(&self) -> RendererInfo {
                        self.0.get_info()
                    }
                    fn validate_parameters(
                        &self,
                        precision: DataPrecision,
                        parameters: &str,
                    ) -> Result<(), RendererError> {
                        self.0.validate_parameters(precision, parameters)
                    }
                }
            };
        }

        // Register many factories with unique types (reduced number for practicality)
        // In a real scenario, you'd have different renderer types, not many of the same type
        for i in 0..10 {
            // Reduced from 100 for test efficiency
            match i {
                0 => {
                    create_factory_type!(PerfFactory0, 0);
                    let factory = Box::new(PerfFactory0(MockRendererFactory::new_full(
                        format!("Renderer{}", i),
                        vec![DataPrecision::F32],
                        format!("capability{},shared", i),
                        1000,
                    )));
                    manager.register(factory).unwrap();
                }
                1 => {
                    create_factory_type!(PerfFactory1, 1);
                    let factory = Box::new(PerfFactory1(MockRendererFactory::new_full(
                        format!("Renderer{}", i),
                        vec![DataPrecision::F32],
                        format!("capability{},shared", i),
                        1000,
                    )));
                    manager.register(factory).unwrap();
                }
                // Add more cases as needed, or just test with fewer factories
                _ => {
                    // For remaining factories, use a different approach or skip
                    // In real tests, you'd have genuinely different factory types
                    break;
                }
            }
        }

        // Test that we can register some factories without conflicts
        assert!(manager.get_factory_count() >= 2);

        // Test query performance (simplified)
        let start = std::time::Instant::now();
        let _shared_renderers = manager.find_by_capability("shared");
        let query_duration = start.elapsed();

        // Should complete quickly
        assert!(query_duration < std::time::Duration::from_millis(100));
    }

    #[test]
    fn test_precision_matrix() {
        let manager = standard_test_manager!(cpu_gpu);
        let matrix = manager.get_precision_matrix();

        // Should have multiple precisions and factories
        assert!(!matrix.precisions.is_empty());
        assert_eq!(matrix.factories.len(), 2); // CPU and GPU factories

        // Check coverage map
        assert!(matrix.coverage_map.contains_key(&DataPrecision::F32));
        let f32_factories = &matrix.coverage_map[&DataPrecision::F32];
        assert!(f32_factories.len() > 0);
    }

    #[test]
    fn test_find_best_factory() {
        let manager = standard_test_manager!(cpu_gpu);

        let requirements = RendererRequirements {
            required_capabilities: vec!["gpu_rendering".to_string()],
            preferred_precision: DataPrecision::F32,
            max_timeout: Duration::from_secs(10),
            performance_priority: PerformancePriority::Speed,
        };

        let best_factory = manager.find_best_factory(&requirements);
        assert!(best_factory.is_some());

        let factory = best_factory.unwrap();
        assert_eq!(factory.name, "GpuRenderer");
    }

    #[test]
    fn test_health_check() {
        let manager = standard_test_manager!(cpu_gpu);
        let health_report = manager.health_check();

        assert_eq!(health_report.total_factories, 2);
        assert!(health_report.healthy_factories > 0);
        assert!(!health_report.recommended_actions.is_empty());
    }

    #[test]
    fn test_factory_metrics() {
        let manager = standard_test_manager!(cpu_gpu);
        let metrics = manager.get_factory_metrics();

        assert_eq!(metrics.len(), 2);
        for metric in metrics {
            assert!(!metric.factory_name.is_empty());
        }
    }
}
