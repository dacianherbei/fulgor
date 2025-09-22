use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::fmt::Debug;
use crate::renderer::{DataPrecision, RendererError};

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

/// Simple mock Renderer trait for testing the factory system
///
/// This is a simplified version of the main Renderer trait, focused on
/// the basic lifecycle methods needed for factory testing.
pub trait Renderer: Send + Sync + Debug + Any {
    /// Start the renderer
    fn start(&mut self) -> Result<(), String>;

    /// Stop the renderer
    fn stop(&mut self);

    /// Get the renderer's name
    fn name(&self) -> &'static str;
}

/// Factory trait for creating renderer instances
///
/// This trait provides a unified interface for creating different types of renderers
/// with configurable precision and parameters. Implementations should be thread-safe
/// and support concurrent creation of multiple renderer instances.
pub trait RendererFactory: Send + Sync + Any {
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
    name: &'static str,
    started: bool,
    precision: DataPrecision,
}

impl MockRenderer {
    pub fn new(name: &'static str, precision: DataPrecision) -> Self {
        Self {
            name,
            started: false,
            precision,
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

#[cfg(test)]
mod tests {
    use super::*;

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
        let any_ref = renderer_f16.as_ref() as &dyn Any;
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
}