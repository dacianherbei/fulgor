use std::any::TypeId;
use std::collections::HashMap;

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
}