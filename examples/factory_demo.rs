// Fixed factory_demo.rs - Updated to work with current OpenGL3RendererConfig structure

use fulgor::renderer::{
    manager::RendererManager,
    factory::{RendererFactory, MockRendererFactory},
    custom::opengl3::{OpenGL3RendererFactory, OpenGL3Renderer, OpenGL3RendererConfig, OpenGL3RendererBuilder},
    prelude::*,
    DataPrecision
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Fulgor Renderer Factory Demonstration ===\n");

    // Create a renderer manager
    let mut manager = RendererManager::new();
    println!("✓ Created RendererManager");

    // Register different renderer factories
    register_factories(&mut manager)?;

    // Demonstrate factory discovery
    demonstrate_factory_discovery(&manager);

    // Demonstrate renderer creation through factories
    demonstrate_renderer_creation(&manager)?;

    // Demonstrate configuration and builders
    demonstrate_configuration_builders()?;

    // Demonstrate direct renderer creation
    demonstrate_direct_creation()?;

    println!("\n=== Factory Demo Completed Successfully ===");
    Ok(())
}

fn register_factories(manager: &mut RendererManager) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n--- Registering Renderer Factories ---");

    // Register Mock renderer factory
    let mock_factory = Box::new(MockRendererFactory::new("MockRenderer"));
    match manager.register(mock_factory) {
        Ok(_) => println!("✓ Registered MockRendererFactory"),
        Err(e) => println!("✗ Failed to register MockRenderer: {}", e),
    }

    // Register OpenGL3 renderer factory
    let opengl_factory = Box::new(OpenGL3RendererFactory::new());
    match manager.register(opengl_factory) {
        Ok(_) => println!("✓ Registered OpenGL3RendererFactory"),
        Err(e) => println!("✗ Failed to register OpenGL3Renderer: {}", e),
    }

    println!("Total factories registered: {}", manager.get_factory_count());
    Ok(())
}

fn demonstrate_factory_discovery(manager: &RendererManager) {
    println!("\n--- Factory Discovery ---");

    // Get all available renderers
    let renderer_infos = manager.get_renderer_info_list();
    println!("Found {} renderer types:", renderer_infos.len());

    for info in &renderer_infos {
        println!("  • {} (timeout: {}ms)", info.name, info.timeout_microseconds);

        // Show capabilities
        let capabilities = info.get_capabilities();
        if !capabilities.is_empty() {
            println!("    Capabilities: {}", capabilities.join(", "));
        }

        // Show parameters if any
        let params = info.get_parameters();
        if !params.is_empty() {
            println!("    Parameters: {}", params.keys().map(|s| s.as_str()).collect::<Vec<_>>().join(", "));
        }
    }

    // Find renderers by capability
    println!("\nRenderers with 'gpu_rendering' capability:");
    let gpu_renderers = manager.find_by_capability("gpu_rendering");
    for info in gpu_renderers {
        println!("  • {}", info.name);
    }
}

fn demonstrate_renderer_creation(manager: &RendererManager) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n--- Renderer Creation Through Manager ---");

    // Create Mock renderer
    println!("Creating Mock renderer...");
    match manager.create_by_name("MockRenderer", DataPrecision::F32, "test_mode=true") {
        Ok((renderer, _rx, _tx)) => {
            println!("✓ Mock renderer created: {}", renderer.name());
            println!("  - Precision: {:?}", renderer.get_data_precision());
            println!("  - Unique ID: {}", renderer.unique_id());
        }
        Err(e) => println!("✗ Failed to create Mock renderer: {}", e),
    }

    // Create OpenGL3 renderer with parameters
    println!("\nCreating OpenGL3 renderer...");
    let opengl_params = "opengl_version=4.1,max_splat_count=500000,msaa_samples=4,viewport_size=1920x1080";
    match manager.create_by_name("OpenGL3Renderer", DataPrecision::F32, opengl_params) {
        Ok((renderer, _rx, _tx)) => {
            println!("✓ OpenGL3 renderer created: {}", renderer.name());
            println!("  - Precision: {:?}", renderer.get_data_precision());
            println!("  - Unique ID: {}", renderer.unique_id());
        }
        Err(e) => println!("✗ Failed to create OpenGL3 renderer: {}", e),
    }

    Ok(())
}

fn demonstrate_configuration_builders() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n--- Configuration and Builders ---");

    // Demonstrate OpenGL3RendererConfig creation and inspection
    println!("Creating custom OpenGL3 configuration...");

    let custom_config = OpenGL3RendererConfig::from_parameters(
        DataPrecision::F16,
        "opengl_version=4.2,max_splat_count=750000,msaa_samples=8,viewport_size=2560x1440"
    )?;

    println!("✓ Custom OpenGL3 configuration created:");
    println!("  - OpenGL Version: {:?}", custom_config.opengl_version);
    println!("  - Max Splat Count: {}", custom_config.max_splat_count);
    println!("  - MSAA Samples: {}", custom_config.msaa_samples);
    println!("  - Viewport: {}x{}", custom_config.viewport_size.0, custom_config.viewport_size.1);
    println!("  - Preferred Precision: {:?}", custom_config.preferred_precision);
    println!("  - Depth Testing: {}", custom_config.depth_testing);
    println!("  - Alpha Blending: {}", custom_config.alpha_blending);

    // Demonstrate OpenGL3RendererBuilder
    println!("\nUsing OpenGL3RendererBuilder...");
    let builder_renderer = OpenGL3RendererBuilder::new()
        .opengl_version(4, 5).map_err(|e| format!("OpenGL version error: {}", e))?
        .max_splat_count(2_000_000)
        .msaa_samples(16).map_err(|e| format!("MSAA error: {}", e))?
        .viewport_size(3840, 2160)
        .preferred_precision(DataPrecision::F32)
        .depth_testing(true)
        .alpha_blending(true)
        .opengl_parameter("vsync", "true")
        .opengl_parameter("debug_context", "false")
        .build();

    println!("✓ Builder renderer created:");
    println!("  - Name: {}", builder_renderer.name());
    println!("  - OpenGL Version Required: {:?}", builder_renderer.required_opengl_version());
    println!("  - Max Splat Count: {}", builder_renderer.max_splat_count());
    println!("  - Viewport Size: {:?}", builder_renderer.viewport_size());
    println!("  - Context Initialized: {}", builder_renderer.is_context_initialized());

    Ok(())
}

fn demonstrate_direct_creation() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n--- Direct Renderer Creation ---");

    // Create renderer with default configuration
    println!("Creating OpenGL3 renderer with default config...");
    let default_renderer = OpenGL3Renderer::default();

    println!("✓ Default OpenGL3 renderer created:");
    println!("  - Name: {}", default_renderer.name());
    println!("  - Precision: {:?}", default_renderer.get_data_precision());
    println!("  - Running: {}", default_renderer.is_running());
    println!("  - Frame Count: {}", default_renderer.get_frame_count());

    // Create renderer with custom configuration
    println!("\nCreating OpenGL3 renderer with custom config...");
    let mut custom_config = OpenGL3RendererConfig::default();
    custom_config.preferred_precision = DataPrecision::F16;
    custom_config.max_splat_count = 500_000;
    custom_config.viewport_size = (1280, 720);
    custom_config.msaa_samples = 2;

    let custom_renderer = OpenGL3Renderer::new(custom_config);

    println!("✓ Custom OpenGL3 renderer created:");
    println!("  - Name: {}", custom_renderer.name());
    println!("  - Precision: {:?}", custom_renderer.get_data_precision());
    println!("  - Max Splats: {}", custom_renderer.max_splat_count());
    println!("  - Viewport: {:?}", custom_renderer.viewport_size());

    Ok(())
}

// Additional utility functions for the demo

fn demonstrate_renderer_capabilities() {
    println!("\n--- Renderer Capabilities ---");

    let opengl_renderer = OpenGL3Renderer::default();

    println!("OpenGL3Renderer capabilities:");
    println!("  - Capability name: {}", opengl_renderer.capability_name());
    if let Some(desc) = opengl_renderer.description() {
        println!("  - Description: {}", desc);
    }

    // Test precision support
    let precisions = [DataPrecision::F16, DataPrecision::F32, DataPrecision::F64, DataPrecision::BFloat16];
    for precision in precisions {
        let supported = opengl_renderer.supports_precision(precision);
        println!("  - {:?}: {}", precision, if supported { "✓" } else { "✗" });
    }

    let supported_precisions = opengl_renderer.supported_precisions();
    println!("  - Supported precisions: {:?}", supported_precisions);

    if let Some(preferred) = opengl_renderer.preferred_precision() {
        println!("  - Preferred precision: {:?}", preferred);
    }
}

fn demonstrate_error_handling() {
    println!("\n--- Error Handling ---");

    // Try creating with invalid OpenGL version
    match OpenGL3RendererBuilder::new().opengl_version(2, 1) {
        Ok(_) => println!("✗ Should have failed with OpenGL 2.1"),
        Err(e) => println!("✓ Correctly rejected OpenGL 2.1: {}", e),
    }

    // Try invalid MSAA samples
    match OpenGL3RendererBuilder::new().msaa_samples(3) {
        Ok(_) => println!("✗ Should have failed with MSAA 3"),
        Err(e) => println!("✓ Correctly rejected MSAA 3: {}", e),
    }

    // Try invalid precision through factory
    let factory = OpenGL3RendererFactory::new();
    match factory.validate_parameters(DataPrecision::F64, "") {
        Ok(_) => println!("✗ Should have failed with F64 precision"),
        Err(e) => println!("✓ Correctly rejected F64: {}", e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_factory_demo_main() {
        // Test that main function runs without panicking
        assert!(main().is_ok());
    }

    #[test]
    fn test_opengl_config_creation() {
        let config = OpenGL3RendererConfig::default();
        assert_eq!(config.opengl_version, (3, 3));
        assert_eq!(config.preferred_precision, DataPrecision::F32);
        assert_eq!(config.max_splat_count, 1_000_000);
    }

    #[test]
    fn test_renderer_builder() {
        let result = OpenGL3RendererBuilder::new()
            .max_splat_count(500_000)
            .preferred_precision(DataPrecision::F16)
            .build();

        assert_eq!(result.get_data_precision(), DataPrecision::F16);
        assert_eq!(result.max_splat_count(), 500_000);
    }
}