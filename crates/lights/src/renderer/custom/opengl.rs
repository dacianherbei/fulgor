use std::collections::HashMap;
use crate::renderer::{DataPrecision, Renderer, RendererError, RendererFactory, RendererInfo};
use crate::renderer::custom::OpenGL3RendererFactory;

/// Version-agnostic OpenGL renderer factory.
///
/// This factory parses the OpenGL version from parameters and delegates
/// to the appropriate version-specific factory.
#[derive(Debug)]
pub struct OpenGLRendererFactory {
    factory_name: String,
}

impl OpenGLRendererFactory {
    pub fn new() -> Self {
        Self {
            factory_name: "OpenGLRenderer".to_string(),
        }
    }
}

impl Default for OpenGLRendererFactory {
    fn default() -> Self {
        Self::new()
    }
}

impl RendererFactory for OpenGLRendererFactory {
    fn create(&self, precision: DataPrecision, parameters: &str) -> Result<Box<dyn Renderer>, RendererError> {
        // Parse OpenGL version from parameters
        let (major, minor, remaining_params) = self.parse_opengl_version(parameters)?;

        // Dispatch to version-specific factory
        match major {
            3 => {
                let factory = OpenGL3RendererFactory::new();
                factory.create(precision, &remaining_params)
            }
            4 => {
                Err(RendererError::CreationFailed(
                    "OpenGL 4.x renderer not yet implemented".to_string()
                ))
            }
            _ => {
                Err(RendererError::InvalidParameters(
                    format!("Unsupported OpenGL version {}.{}", major, minor)
                ))
            }
        }
    }

    fn get_info(&self) -> RendererInfo {
        let mut parameters = HashMap::new();

        parameters.insert(
            "opengl_version".to_string(),
            "OpenGL version (e.g., '3.3', '4.2') - determines which renderer to use".to_string()
        );
        parameters.insert(
            "max_splat_count".to_string(),
            "Maximum number of Gaussian splats to support".to_string()
        );
        parameters.insert(
            "msaa_samples".to_string(),
            "MSAA sample count (0, 2, 4, 8, 16)".to_string()
        );
        parameters.insert(
            "viewport_size".to_string(),
            "Viewport dimensions in WxH format (e.g., '1920x1080')".to_string()
        );
        parameters.insert(
            "depth_testing".to_string(),
            "Enable depth testing (true/false)".to_string()
        );
        parameters.insert(
            "alpha_blending".to_string(),
            "Enable alpha blending (true/false)".to_string()
        );

        RendererInfo::new(
            self.factory_name.clone(),
            "opengl,gpu_acceleration,hardware_rendering,version_agnostic,gaussian_splatting,real_time".to_string(),
            parameters,
            15000, // 15ms timeout (higher since it delegates)
        )
    }

    fn validate_parameters(&self, precision: DataPrecision, parameters: &str) -> Result<(), RendererError> {
        let (major, minor, remaining_params) = self.parse_opengl_version(parameters)?;

        match major {
            3 => {
                let factory = OpenGL3RendererFactory::new();
                factory.validate_parameters(precision, &remaining_params)
            }
            4 => {
                Err(RendererError::InvalidParameters(
                    "OpenGL 4.x renderer not yet implemented".to_string()
                ))
            }
            _ => {
                Err(RendererError::InvalidParameters(
                    format!("Unsupported OpenGL version {}.{}", major, minor)
                ))
            }
        }
    }
}

impl OpenGLRendererFactory {
    /// Parse OpenGL version from parameter string and return (major, minor, remaining_params)
    fn parse_opengl_version(&self, parameters: &str) -> Result<(u32, u32, String), RendererError> {
        let params = crate::renderer::factory::parse_parameters(parameters);

        // Find opengl_version parameter (default to 3.3 if not specified)
        let version_str = params.get("opengl_version")
            .map(|s| s.as_str())
            .unwrap_or("3.3");

        // Parse version string
        let parts: Vec<&str> = version_str.split('.').collect();
        if parts.len() != 2 {
            return Err(RendererError::InvalidParameters(
                format!("Invalid OpenGL version format '{}'. Expected format: 'major.minor' (e.g., '3.3')", version_str)
            ));
        }

        let major = parts[0].parse::<u32>()
            .map_err(|_| RendererError::InvalidParameters(
                format!("Invalid OpenGL major version: {}", parts[0])
            ))?;
        let minor = parts[1].parse::<u32>()
            .map_err(|_| RendererError::InvalidParameters(
                format!("Invalid OpenGL minor version: {}", parts[1])
            ))?;

        // Rebuild parameter string without opengl_version
        let remaining_params: Vec<String> = params.iter()
            .filter(|(key, _)| key.as_str() != "opengl_version")
            .map(|(key, value)| format!("{}={}", key, value))
            .collect();
        let remaining_params = remaining_params.join(",");

        Ok((major, minor, remaining_params))
    }
}