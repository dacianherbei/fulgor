use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::fmt::Debug;
use std::future::Future;
use std::pin::Pin;
use tokio::sync::mpsc::{UnboundedReceiver};
pub(crate) use crate::renderer::{DataPrecision, Renderer, RendererError};
use crate::renderer::{generate_renderer_id, BufferedAsyncSender, RendererEvent};

pub fn parse_parameters(parameters: &str) -> HashMap<String, String> {
    let mut result = HashMap::new();

    if parameters.is_empty() {
        return result;
    }

    for pair in parameters.split(',') {
        let pair = pair.trim();
        if pair.is_empty() {
            continue;
        }

        if let Some((key, value)) = pair.split_once('=') {
            let key = key.trim().to_string();
            let value = value.trim().to_string();
            if !key.is_empty() {
                result.insert(key, value);
            }
        } else {
            // Handle parameters without values as boolean flags
            result.insert(pair.to_string(), "true".to_string());
        }
    }

    result
}

/// Information about a registered renderer in the factory system.
///
/// This struct contains metadata about a renderer that can be used by the
/// RendererManager to describe capabilities, parameters, and creation details
/// for each registered renderer type.
#[derive(Debug, Clone)]
pub struct RendererInfo {
    /// Unique type identifier set by RendererManager during registration
    pub id: TypeId,

    /// Human-readable name of the renderer
    pub name: String,

    /// Comma-separated list of capabilities supported by this renderer
    pub capabilities: String,

    /// Map of parameter names to their descriptions
    pub parameters: HashMap<String, String>,

    /// Maximum time in microseconds allowed for renderer creation
    pub timeout_microseconds: u64,
}

impl RendererInfo {
    /// Create a new RendererInfo instance.
    ///
    /// Note: The `id` field will typically be set by the RendererManager
    /// during registration, so this constructor sets it to a placeholder.
    pub fn new(
        name: String,
        capabilities: String,
        parameters: HashMap<String, String>,
        timeout_microseconds: u64,
    ) -> Self {
        Self {
            id: TypeId::of::<()>(), // Placeholder - will be set during registration
            name,
            capabilities,
            parameters,
            timeout_microseconds,
        }
    }

    /// Split the capabilities string into individual capability names.
    ///
    /// Returns a vector of capability strings, with whitespace trimmed.
    /// Empty capabilities after trimming are filtered out.
    pub fn get_capabilities(&self) -> Vec<&str> {
        self.capabilities
            .split(',')
            .map(|cap| cap.trim())
            .filter(|cap| !cap.is_empty())
            .collect()
    }

    /// Check if this renderer supports a specific capability.
    ///
    /// The check is case-sensitive and matches against trimmed capability names.
    pub fn has_capability(&self, capability: &str) -> bool {
        self.get_capabilities()
            .iter()
            .any(|&cap| cap == capability.trim())
    }

    /// Set the TypeId (typically called by RendererManager during registration).
    pub fn set_id(&mut self, id: TypeId) {
        self.id = id;
    }

    /// Get a reference to the parameters map.
    pub fn get_parameters(&self) -> &HashMap<String, String> {
        &self.parameters
    }

    /// Check if a parameter is supported by this renderer.
    pub fn has_parameter(&self, param_name: &str) -> bool {
        self.parameters.contains_key(param_name)
    }

    /// Get the description for a specific parameter, if it exists.
    pub fn get_parameter_description(&self, param_name: &str) -> Option<&String> {
        self.parameters.get(param_name)
    }
}

/// Factory trait for creating renderer instances
///
/// This trait provides a unified interface for creating different types of renderers
/// with configurable precision and parameters. Implementations should be thread-safe
/// and support concurrent creation of multiple renderer instances.
pub trait RendererFactory: Send + Sync + Any + Debug {
    /// Create a new renderer instance with the specified precision and parameters
    ///
    /// # Arguments
    /// * `precision` - The data precision to use for rendering calculations
    /// * `parameters` - Factory-specific configuration parameters as a string
    ///
    /// # Returns
    /// A boxed renderer instance or an error if creation failed
    fn create(&self, precision: DataPrecision, parameters: &str) -> Result<Box<dyn Renderer>, RendererError>;

    /// Get information about this renderer factory
    ///
    /// # Returns
    /// RendererInfo describing the capabilities and requirements of this factory
    fn get_info(&self) -> RendererInfo;

    /// Validate parameters without creating a renderer instance
    ///
    /// This method allows checking if parameters are valid before attempting
    /// to create an expensive renderer instance. The default implementation
    /// always returns Ok(()), but factories should override this for proper validation.
    ///
    /// # Arguments
    /// * `precision` - The data precision to validate against
    /// * `parameters` - The parameters string to validate
    ///
    /// # Returns
    /// Ok(()) if parameters are valid, or a RendererError describing the issue
    fn validate_parameters(&self, _precision: DataPrecision, _parameters: &str) -> Result<(), RendererError> {
        Ok(())
    }
}

/// Mock renderer implementation for testing
#[derive(Debug)]
pub struct MockRenderer {
    /// Unique ID for this renderer instance - generated once, never changes
    id: u64,
    name: &'static str,
    started: bool,
    precision: DataPrecision,

    sender: BufferedAsyncSender<RendererEvent>,
    receiver: UnboundedReceiver<RendererEvent>
}

impl MockRenderer {
    pub fn new(name: &'static str, precision: DataPrecision) -> Self {
        let id = generate_renderer_id();
        let (buffered_sender, buffered_receiver) = BufferedAsyncSender::<RendererEvent>::new_unbounded(Option::<usize>::Some(100));
        Self {
            id,
            name,
            started: false,
            precision,
            sender: buffered_sender,
            receiver: buffered_receiver
        }
    }

    pub fn precision(&self) -> DataPrecision {
        self.precision
    }

    pub fn is_started(&self) -> bool {
        self.started
    }
}

impl Renderer for MockRenderer {
    fn unique_id(&self) -> u64 {
        self.id  // ← Simply return the stored ID
    }

    fn shutdown_timeout(&self) -> std::time::Duration {
        std::time::Duration::from_millis(1000) // 1 second for reference renderer
    }

    fn set_data_precision(&mut self, precision: DataPrecision) -> Result<DataPrecision, String> {
        todo!()
    }

    fn get_data_precision(&self) -> DataPrecision {
        todo!()
    }

    fn is_running(&self) -> bool {
        todo!()
    }

    fn get_frame_count(&self) -> u64 {
        todo!()
    }

    fn start(&mut self) -> Result<(), String> {
        if self.started {
            Err("Renderer is already started".to_string())
        } else {
            self.started = true;
            Ok(())
        }
    }

    fn stop(&mut self) {
        self.started = false;
    }

    fn name(&self) -> &'static str {
        self.name
    }

    fn render_frame(&mut self) -> Result<(), String> {
        todo!()
    }

    fn sender(&self) -> BufferedAsyncSender<RendererEvent> {
        self.sender.clone()
    }

    fn run(&mut self) -> Pin<Box<dyn Future<Output = ()> + Send + '_>> {
        Box::pin(async move {
            while let Some(event) = self.receiver.recv().await {
                match event {
                    RendererEvent::Shutdown(id) => {
                        println!("MockRenderer shut down {:?}", id);
                        break;
                    }
                    RendererEvent::Started(id) => {
                        println!("MockRenderer started {:?}", id);
                    }
                    RendererEvent::Stopped(id) => {
                        println!("MockRenderer stopped {:?}", id);
                    }
                    RendererEvent::Switched(active) => {
                        println!("MockRenderer switched {:?}", active);
                    }
                    other => {
                        println!("MockRenderer ignoring event {:?}", other);
                    }
                }
            }
        })
    }
}

/// Simple example factory implementation for testing
#[derive(Debug)]
pub struct MockRendererFactory {
    factory_name: String,
    supported_precisions: Vec<DataPrecision>,
    capabilities: String,
    timeout_microseconds: u64,
}

impl MockRendererFactory {
    pub fn new(factory_name: impl Into<String>) -> Self {
        Self {
            factory_name: factory_name.into(),
            supported_precisions: vec![DataPrecision::F32, DataPrecision::F64],
            capabilities: "testing,mock,basic_rendering".to_string(),
            timeout_microseconds: 5000,
        }
    }

    pub fn new_with_precisions(
        factory_name: impl Into<String>,
        supported_precisions: Vec<DataPrecision>
    ) -> Self {
        Self {
            factory_name: factory_name.into(),
            supported_precisions,
            capabilities: "testing,mock,configurable_precision".to_string(),
            timeout_microseconds: 3000,
        }
    }

    pub fn new_full(
        factory_name: impl Into<String>,
        supported_precisions: Vec<DataPrecision>,
        capabilities: impl Into<String>,
        timeout_microseconds: u64,
    ) -> Self {
        Self {
            factory_name: factory_name.into(),
            supported_precisions,
            capabilities: capabilities.into(),
            timeout_microseconds,
        }
    }
}

impl RendererFactory for MockRendererFactory {
    fn create(&self, precision: DataPrecision, parameters: &str) -> Result<Box<dyn Renderer>, RendererError> {
        // Validate precision support
        if !self.supported_precisions.contains(&precision) {
            return Err(RendererError::UnsupportedPrecision(precision));
        }

        // Validate parameters first
        self.validate_parameters(precision, parameters)?;

        // Create the mock renderer
        let renderer_name = if parameters.contains("custom_name=") {
            "CustomMock"
        } else {
            "Mock"
        };

        Ok(Box::new(MockRenderer::new(renderer_name, precision)))
    }

    fn get_info(&self) -> RendererInfo {
        let mut parameters = HashMap::new();
        parameters.insert("custom_name".to_string(), "Set to 'true' to use CustomMock renderer name".to_string());
        parameters.insert("test_mode".to_string(), "Enable test mode for debugging".to_string());

        RendererInfo::new(
            self.factory_name.clone(),
            self.capabilities.clone(),
            parameters,
            self.timeout_microseconds,
        )
    }

    fn validate_parameters(&self, _precision: DataPrecision, parameters: &str) -> Result<(), RendererError> {
        // Simple parameter validation
        if parameters.contains("invalid") {
            return Err(RendererError::InvalidParameters(
                "Parameters cannot contain 'invalid'".to_string()
            ));
        }

        if parameters.len() > 100 {
            return Err(RendererError::InvalidParameters(
                "Parameters too long (max 100 characters)".to_string()
            ));
        }

        // Validate specific parameter formats
        if parameters.contains("custom_name=") && !parameters.contains("custom_name=true") {
            return Err(RendererError::InvalidParameters(
                "custom_name parameter must be 'true' if specified".to_string()
            ));
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct ReferenceRendererConfig {
    pub threads: usize,
    pub quality: String,
    pub debug: bool,
    /// Preferred data precision for vertex data
    pub precision: DataPrecision,
    /// Maximum number of Gaussian splats to render
    pub max_splat_count: usize,

    /// Supported data precisions
    pub supported_precisions: Vec<DataPrecision>,

    /// Viewport dimensions (width, height)
    pub viewport_size: (u32, u32),
}

impl Default for ReferenceRendererConfig {
    fn default() -> Self {
        Self {
            threads: num_cpus::get(),
            quality: "medium".to_string(),
            debug: false,
            precision: DataPrecision::F32,
            max_splat_count: 1_000_000, // 1 million splats
            supported_precisions: vec![DataPrecision::F16, DataPrecision::F32],
            viewport_size: (1920, 1080), // Default to 1080p
        }
    }
}

impl ReferenceRendererConfig {
    /// Create configuration from parameter string.
    pub fn from_parameters(precision: DataPrecision, parameters: &str) -> Result<Self, String> {
        let mut config = Self::default();
        config.precision = precision;

        if parameters.is_empty() {
            return Ok(config);
        }

        let params = crate::renderer::factory::parse_parameters(parameters);

        for (key, value) in params {
            match key.as_str() {
                "max_splat_count" => {
                    config.max_splat_count = value.parse::<usize>()
                        .map_err(|_| format!("Invalid max_splat_count: {}", value))?;
                }
                "threads" => {
                    config.threads = value.parse::<usize>()
                        .map_err(|_| format!("Invalid threads value: {}", value))?;

                    if config.threads == 0 {
                        return Err("threads must be greater than 0".to_string());
                    }
                }
                "quality" => {
                    config.quality = value.to_string();
                    match config.quality.as_str() {
                        "low" | "medium" | "high" | "ultra" => {
                            // Valid quality values
                        }
                        _ => {
                            return Err(format!(
                                "Invalid quality value: {}. Must be one of: low, medium, high, ultra",
                                config.quality
                            ));
                        }
                    }
                }
                "debug" => {
                    config.debug = value.parse::<bool>()
                        .map_err(|_| format!("Invalid debug value: {}", value))?;
                }
                "viewport_size" => {
                    let parts: Vec<&str> = value.split('x').collect();
                    if parts.len() == 2 {
                        let width = parts[0].parse::<u32>()
                            .map_err(|_| format!("Invalid viewport width: {}", parts[0]))?;
                        let height = parts[1].parse::<u32>()
                            .map_err(|_| format!("Invalid viewport height: {}", parts[1]))?;
                        config.viewport_size = (width, height);
                    } else {
                        return Err(format!(
                            "Invalid viewport_size format: {}. Expected format: 'widthxheight' (e.g., '1920x1080')",
                            value
                        ));
                    }
                }
                _ => {
                    // Unknown parameters are ignored (could be made strict by uncommenting below)
                    // return Err(format!("Unknown parameter: {}", key));
                }
            }
        }

        Ok(config)
    }


    /// Validate that the configuration is valid
    pub fn validate(&self) -> Result<(), String> {
        if self.threads == 0 {
            return Err("threads must be greater than 0".to_string());
        }

        if self.max_splat_count == 0 {
            return Err("max_splat_count must be greater than 0".to_string());
        }

        if self.viewport_size.0 == 0 || self.viewport_size.1 == 0 {
            return Err("viewport_size width and height must be greater than 0".to_string());
        }

        match self.quality.as_str() {
            "low" | "medium" | "high" | "ultra" => {}
            _ => {
                return Err(format!(
                    "Invalid quality: {}. Must be one of: low, medium, high, ultra",
                    self.quality
                ));
            }
        }

        Ok(())
    }

    pub fn description(&self) -> String {
        format!(
            "ReferenceRenderer Config: {}x{} viewport, {} threads, {} quality, precision: {:?}, max_splats: {}, debug: {}",
            self.viewport_size.0,
            self.viewport_size.1,
            self.threads,
            self.quality,
            self.precision,
            self.max_splat_count,
            self.debug
        )
    }
}

/// Factory for creating ReferenceRenderer instances.
#[derive(Debug)]
pub struct ReferenceRendererFactory {
    factory_name: String,
}

impl ReferenceRendererFactory {
    pub fn new() -> Self {
        Self {
            factory_name: "ReferenceRenderer".to_string(),
        }
    }
}

impl Default for ReferenceRendererFactory {
    fn default() -> Self {
        Self::new()
    }
}

impl RendererFactory for ReferenceRendererFactory {
    fn create(&self, precision: DataPrecision, parameters: &str) -> Result<Box<dyn crate::renderer::Renderer>, RendererError> {
        // Parse parameters if any
        if !parameters.is_empty() {
            let params = parse_parameters(parameters);
            for (key, _) in params {
                match key.as_str() {
                    "precision" => {}, // Handled by precision parameter
                    _ => {
                        return Err(RendererError::InvalidParameters(
                            format!("Unknown parameter for ReferenceRenderer: {}", key)
                        ));
                    }
                }
            }
        }

        Ok(Box::new(crate::renderer::ReferenceRenderer::with_precision(precision)))
    }

    fn get_info(&self) -> RendererInfo {
        let mut parameters = std::collections::HashMap::new();
        parameters.insert(
            "precision".to_string(),
            "Data precision for rendering (f16, f32, f64, bfloat16)".to_string()
        );

        RendererInfo::new(
            self.factory_name.clone(),
            "reference,cpu,basic_rendering,all_precisions".to_string(),
            parameters,
            1000, // 1ms timeout
        )
    }

    fn validate_parameters(&self, _precision: DataPrecision, parameters: &str) -> Result<(), RendererError> {
        if parameters.is_empty() {
            return Ok(());
        }

        let params = parse_parameters(parameters);
        for (key, _) in params {
            match key.as_str() {
                "precision" => {},
                _ => {
                    return Err(RendererError::InvalidParameters(
                        format!("Unknown parameter for ReferenceRenderer: {}", key)
                    ));
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::any::TypeId;

    #[test]
    fn test_renderer_info_creation() {
        let mut params = HashMap::new();
        params.insert("buffer_size".to_string(), "Size of the rendering buffer".to_string());
        params.insert("quality".to_string(), "Rendering quality level (1-10)".to_string());

        let info = RendererInfo::new(
            "TestRenderer".to_string(),
            "3d_rendering,gaussian_splatting,real_time".to_string(),
            params,
            5000, // 5ms timeout
        );

        assert_eq!(info.name, "TestRenderer");
        assert_eq!(info.timeout_microseconds, 5000);
        assert_eq!(info.parameters.len(), 2);
    }

    #[test]
    fn test_get_capabilities() {
        let info = RendererInfo::new(
            "TestRenderer".to_string(),
            "3d_rendering, gaussian_splatting,real_time , optimization".to_string(),
            HashMap::new(),
            1000,
        );

        let capabilities = info.get_capabilities();
        assert_eq!(capabilities.len(), 4);
        assert!(capabilities.contains(&"3d_rendering"));
        assert!(capabilities.contains(&"gaussian_splatting"));
        assert!(capabilities.contains(&"real_time"));
        assert!(capabilities.contains(&"optimization"));
    }

    #[test]
    fn test_get_capabilities_with_empty_entries() {
        let info = RendererInfo::new(
            "TestRenderer".to_string(),
            "valid_cap, , another_cap,  ".to_string(),
            HashMap::new(),
            1000,
        );

        let capabilities = info.get_capabilities();
        assert_eq!(capabilities.len(), 2);
        assert!(capabilities.contains(&"valid_cap"));
        assert!(capabilities.contains(&"another_cap"));
    }

    #[test]
    fn test_has_capability() {
        let info = RendererInfo::new(
            "TestRenderer".to_string(),
            "3d_rendering,gaussian_splatting,real_time".to_string(),
            HashMap::new(),
            1000,
        );

        assert!(info.has_capability("3d_rendering"));
        assert!(info.has_capability("gaussian_splatting"));
        assert!(info.has_capability("real_time"));
        assert!(!info.has_capability("invalid_capability"));
        assert!(!info.has_capability("3D_RENDERING")); // Case sensitive
    }

    #[test]
    fn test_has_capability_with_whitespace() {
        let info = RendererInfo::new(
            "TestRenderer".to_string(),
            "capability_one, capability_two ".to_string(),
            HashMap::new(),
            1000,
        );

        assert!(info.has_capability("capability_one"));
        assert!(info.has_capability("capability_two"));
        assert!(info.has_capability(" capability_one ")); // Trimmed during check
    }

    #[test]
    fn test_parameter_methods() {
        let mut params = HashMap::new();
        params.insert("width".to_string(), "Render width in pixels".to_string());
        params.insert("height".to_string(), "Render height in pixels".to_string());

        let info = RendererInfo::new(
            "TestRenderer".to_string(),
            "rendering".to_string(),
            params,
            1000,
        );

        assert!(info.has_parameter("width"));
        assert!(info.has_parameter("height"));
        assert!(!info.has_parameter("depth"));

        assert_eq!(
            info.get_parameter_description("width"),
            Some(&"Render width in pixels".to_string())
        );
        assert_eq!(info.get_parameter_description("nonexistent"), None);

        let params_ref = info.get_parameters();
        assert_eq!(params_ref.len(), 2);
    }

    #[test]
    fn test_set_id() {
        let mut info = RendererInfo::new(
            "TestRenderer".to_string(),
            "rendering".to_string(),
            HashMap::new(),
            1000,
        );

        let original_id = info.id;
        let new_id = TypeId::of::<String>();

        info.set_id(new_id);
        assert_ne!(info.id, original_id);
        assert_eq!(info.id, new_id);
    }

    #[test]
    fn test_debug_and_clone() {
        let info = RendererInfo::new(
            "TestRenderer".to_string(),
            "rendering".to_string(),
            HashMap::new(),
            1000,
        );

        // Test Debug trait
        let debug_string = format!("{:?}", info);
        assert!(debug_string.contains("RendererInfo"));
        assert!(debug_string.contains("TestRenderer"));

        // Test Clone trait
        let cloned_info = info.clone();
        assert_eq!(info.name, cloned_info.name);
        assert_eq!(info.capabilities, cloned_info.capabilities);
        assert_eq!(info.timeout_microseconds, cloned_info.timeout_microseconds);
        assert_eq!(info.id, cloned_info.id);
    }

    #[test]
    fn test_empty_capabilities() {
        let info = RendererInfo::new(
            "MinimalRenderer".to_string(),
            "".to_string(),
            HashMap::new(),
            1000,
        );

        let capabilities = info.get_capabilities();
        assert!(capabilities.is_empty());
        assert!(!info.has_capability("anything"));
    }

    #[test]
    fn test_capability_parsing_edge_cases() {
        // Test with only commas and whitespace
        let info1 = RendererInfo::new(
            "TestRenderer".to_string(),
            " , , , ".to_string(),
            HashMap::new(),
            1000,
        );
        assert!(info1.get_capabilities().is_empty());

        // Test with single capability
        let info2 = RendererInfo::new(
            "TestRenderer".to_string(),
            "single_capability".to_string(),
            HashMap::new(),
            1000,
        );
        let caps2 = info2.get_capabilities();
        assert_eq!(caps2.len(), 1);
        assert_eq!(caps2[0], "single_capability");

        // Test with trailing comma
        let info3 = RendererInfo::new(
            "TestRenderer".to_string(),
            "cap1,cap2,".to_string(),
            HashMap::new(),
            1000,
        );
        let caps3 = info3.get_capabilities();
        assert_eq!(caps3.len(), 2);
        assert!(caps3.contains(&"cap1"));
        assert!(caps3.contains(&"cap2"));
    }
    #[test]
    fn test_mock_renderer() {
        let mut renderer = MockRenderer::new("TestMock", DataPrecision::F32);

        assert_eq!(renderer.name(), "TestMock");
        assert_eq!(renderer.precision(), DataPrecision::F32);
        assert!(!renderer.is_started());

        // Test starting
        assert!(renderer.start().is_ok());
        assert!(renderer.is_started());

        // Test double start fails
        assert!(renderer.start().is_err());

        // Test stopping
        renderer.stop();
        assert!(!renderer.is_started());
    }

    #[test]
    fn test_mock_factory_creation() {
        let factory = MockRendererFactory::new("TestFactory");
        let info = factory.get_info();

        assert_eq!(info.name, "TestFactory");
        assert_eq!(info.timeout_microseconds, 5000);
        assert!(info.has_capability("testing"));
        assert!(info.has_capability("mock"));
        assert!(info.has_capability("basic_rendering"));
        assert!(!info.has_capability("gpu_acceleration"));
    }

    #[test]
    fn test_factory_create_renderer() {
        let factory = MockRendererFactory::new("TestFactory");

        // Test successful creation
        let renderer = factory.create(DataPrecision::F32, "test_params");
        assert!(renderer.is_ok());
        let renderer = renderer.unwrap();
        assert_eq!(renderer.name(), "Mock");

        // Test custom name parameter
        let renderer = factory.create(DataPrecision::F64, "custom_name=true");
        assert!(renderer.is_ok());
        let renderer = renderer.unwrap();
        assert_eq!(renderer.name(), "CustomMock");
    }

    #[test]
    fn test_factory_unsupported_precision() {
        let factory = MockRendererFactory::new_with_precisions(
            "LimitedFactory",
            vec![DataPrecision::F32]
        );

        // Should succeed for supported precision
        let result = factory.create(DataPrecision::F32, "test");
        assert!(result.is_ok());

        // Should fail for unsupported precision
        let result = factory.create(DataPrecision::F64, "test");
        assert!(result.is_err());
        match result.unwrap_err() {
            RendererError::UnsupportedPrecision(DataPrecision::F64) => {},
            _ => panic!("Expected UnsupportedPrecision error"),
        }

        // Test BFloat16 and F16 support
        let advanced_factory = MockRendererFactory::new_with_precisions(
            "AdvancedFactory",
            vec![DataPrecision::F16, DataPrecision::BFloat16]
        );

        assert!(advanced_factory.create(DataPrecision::F16, "").is_ok());
        assert!(advanced_factory.create(DataPrecision::BFloat16, "").is_ok());
        assert!(advanced_factory.create(DataPrecision::F32, "").is_err());
    }

    #[test]
    fn test_parameter_validation() {
        let factory = MockRendererFactory::new("TestFactory");

        // Valid parameters
        assert!(factory.validate_parameters(DataPrecision::F32, "valid_params").is_ok());
        assert!(factory.validate_parameters(DataPrecision::F32, "custom_name=true").is_ok());
        assert!(factory.validate_parameters(DataPrecision::F32, "test_mode=debug").is_ok());

        // Invalid parameters
        let result = factory.validate_parameters(DataPrecision::F32, "invalid_params");
        assert!(result.is_err());
        match result.unwrap_err() {
            RendererError::InvalidParameters(_) => {},
            _ => panic!("Expected InvalidParameters error"),
        }

        // Too long parameters
        let long_params = "a".repeat(101);
        let result = factory.validate_parameters(DataPrecision::F32, &long_params);
        assert!(result.is_err());

        // Invalid custom_name format
        let result = factory.validate_parameters(DataPrecision::F32, "custom_name=false");
        assert!(result.is_err());
    }

    #[test]
    fn test_factory_type_id() {
        let factory1 = MockRendererFactory::new("Factory1");
        let factory2 = MockRendererFactory::new("Factory2");

        // Same type should have same TypeId
        assert_eq!(factory1.type_id(), factory2.type_id());
        assert_eq!(factory1.type_id(), TypeId::of::<MockRendererFactory>());
    }

    #[test]
    fn test_trait_send_sync() {
        // Test that our types implement Send + Sync
        fn assert_send_sync<T: Send + Sync>() {}

        assert_send_sync::<MockRendererFactory>();
        assert_send_sync::<Box<dyn RendererFactory>>();
        assert_send_sync::<Box<dyn Renderer>>();
    }

    #[test]
    fn test_renderer_info_integration() {
        let factory = MockRendererFactory::new_full(
            "AdvancedRenderer",
            vec![DataPrecision::F32, DataPrecision::F64, DataPrecision::F16],
            "3d_rendering,gaussian_splatting,real_time,gpu_accelerated",
            2000
        );

        let info = factory.get_info();
        assert_eq!(info.name, "AdvancedRenderer");
        assert_eq!(info.timeout_microseconds, 2000);

        // Test capabilities
        assert!(info.has_capability("3d_rendering"));
        assert!(info.has_capability("gaussian_splatting"));
        assert!(info.has_capability("real_time"));
        assert!(info.has_capability("gpu_accelerated"));
        assert!(!info.has_capability("cpu_only"));

        // Test parameters
        assert!(info.has_parameter("custom_name"));
        assert!(info.has_parameter("test_mode"));
        assert!(!info.has_parameter("nonexistent"));

        // Test parameter descriptions
        assert!(info.get_parameter_description("custom_name").is_some());
        assert!(info.get_parameter_description("nonexistent").is_none());
    }

    #[test]
    fn test_all_data_precision_variants() {
        let factory = MockRendererFactory::new_with_precisions(
            "FullPrecisionFactory",
            vec![DataPrecision::F16, DataPrecision::F32, DataPrecision::F64, DataPrecision::BFloat16]
        );

        // Test all precision variants
        assert!(factory.create(DataPrecision::F16, "").is_ok());
        assert!(factory.create(DataPrecision::F32, "").is_ok());
        assert!(factory.create(DataPrecision::F64, "").is_ok());
        assert!(factory.create(DataPrecision::BFloat16, "").is_ok());

        // Verify the precision is preserved in created renderers
        let renderer_f16 = factory.create(DataPrecision::F16, "").unwrap();
        let any_ref = &renderer_f16.as_ref() as &dyn Any;
        if let Some(mock) = any_ref.downcast_ref::<MockRenderer>() {
            assert_eq!(mock.precision(), DataPrecision::F16);
        } else {
            panic!("Failed to downcast to MockRenderer");
        }
    }

    #[test]
    fn test_error_propagation() {
        let factory = MockRendererFactory::new("TestFactory");

        // Test that validation errors are propagated during creation
        let result = factory.create(DataPrecision::F32, "invalid_test");
        assert!(result.is_err());

        // Test that the error message is preserved
        if let Err(RendererError::InvalidParameters(msg)) = result {
            assert!(msg.contains("invalid"));
        } else {
            panic!("Expected InvalidParameters error with preserved message");
        }
    }

    #[test]
    fn test_parse_parameters() {
        // Test empty parameters
        let params = parse_parameters("");
        assert!(params.is_empty());

        // Test single parameter
        let params = parse_parameters("threads=4");
        assert_eq!(params.get("threads"), Some(&"4".to_string()));

        // Test multiple parameters
        let params = parse_parameters("threads=4,quality=high,debug=true");
        assert_eq!(params.get("threads"), Some(&"4".to_string()));
        assert_eq!(params.get("quality"), Some(&"high".to_string()));
        assert_eq!(params.get("debug"), Some(&"true".to_string()));

        // Test parameters with spaces
        let params = parse_parameters(" threads = 4 , quality = high ");
        assert_eq!(params.get("threads"), Some(&"4".to_string()));
        assert_eq!(params.get("quality"), Some(&"high".to_string()));

        // Test boolean flags (no value)
        let params = parse_parameters("debug,verbose");
        assert_eq!(params.get("debug"), Some(&"true".to_string()));
        assert_eq!(params.get("verbose"), Some(&"true".to_string()));

        // Test mixed parameters
        let params = parse_parameters("threads=8,debug,quality=ultra");
        assert_eq!(params.get("threads"), Some(&"8".to_string()));
        assert_eq!(params.get("debug"), Some(&"true".to_string()));
        assert_eq!(params.get("quality"), Some(&"ultra".to_string()));
    }

    #[test]
    fn test_from_parameters_empty() {
        let config = ReferenceRendererConfig::from_parameters(DataPrecision::F32, "").unwrap();
        assert_eq!(config.precision, DataPrecision::F32);
        assert_eq!(config.quality, "medium");
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_from_parameters_valid() {
        let params = "threads=8,quality=high,debug=true,max_splat_count=2000000,viewport_size=2560x1440";
        let config = ReferenceRendererConfig::from_parameters(DataPrecision::F64, params).unwrap();

        assert_eq!(config.precision, DataPrecision::F64);
        assert_eq!(config.threads, 8);
        assert_eq!(config.quality, "high");
        assert_eq!(config.debug, true);
        assert_eq!(config.max_splat_count, 2_000_000);
        assert_eq!(config.viewport_size, (2560, 1440));
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_from_parameters_invalid_threads() {
        let params = "threads=0";
        let result = ReferenceRendererConfig::from_parameters(DataPrecision::F32, params);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("threads must be greater than 0"));
    }

    #[test]
    fn test_from_parameters_invalid_quality() {
        let params = "quality=extreme";
        let result = ReferenceRendererConfig::from_parameters(DataPrecision::F32, params);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid quality value"));
    }

    #[test]
    fn test_from_parameters_invalid_viewport() {
        let params = "viewport_size=1920";  // Missing height
        let result = ReferenceRendererConfig::from_parameters(DataPrecision::F32, params);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid viewport_size format"));
    }

    #[test]
    fn test_from_parameters_invalid_numeric() {
        let params = "threads=abc";
        let result = ReferenceRendererConfig::from_parameters(DataPrecision::F32, params);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid threads value"));
    }

    #[test]
    fn test_validate_config() {
        let mut config = ReferenceRendererConfig::default();
        assert!(config.validate().is_ok());

        config.threads = 0;
        assert!(config.validate().is_err());

        config.threads = 4;
        config.viewport_size = (0, 1080);
        assert!(config.validate().is_err());

        config.viewport_size = (1920, 1080);
        config.quality = "invalid".to_string();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_description() {
        let config = ReferenceRendererConfig::default();
        let desc = config.description();
        assert!(desc.contains("1920x1080"));
        assert!(desc.contains("medium"));
        assert!(desc.contains("F32"));
    }
}