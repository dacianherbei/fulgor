//! Example usage of the renderer system with capability-based selection.

use fulgor::renderer::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize manager and register all available renderers
    let mut manager = RendererManager::new();

    // Register the 3 core renderers
    manager.register(Box::new(ReferenceRendererFactory::new()))?;
    manager.register(Box::new(OpenGL3RendererFactory::new()))?;
    manager.register(Box::new(MockRendererFactory::new("TestMock")))?;

    println!("=== Available Renderers ===");
    for info in manager.get_renderer_info_list() {
        println!("- {} (capabilities: {})", info.name, info.capabilities);
    }

    // Example 1: Create GPU renderer with fallback
    println!("\n=== GPU Renderer with Fallback ===");
    let (renderer, _rx, _tx) = manager.create_by_name("gpu_acceleration", DataPrecision::F32, "msaa_samples=8")
        .or_else(|_| manager.create_by_name("basic_rendering", DataPrecision::F32, ""))
        .expect("No renderer available");

    println!("Created: {}", renderer.name());
    println!("Precision: {:?}", renderer.get_data_precision());

    // Example 2: Find best renderer for Gaussian splatting
    println!("\n=== Best Gaussian Splatting Renderer ===");
    let best_info = manager.find_factory_by_name("gaussian_splatting")
        .into_iter()
        .min_by_key(|info| info.timeout_microseconds)
        .expect("No Gaussian splatting renderer");

    let (best_renderer, _rx, _tx) = manager.create_by_name(&best_info.name, DataPrecision::F32, "")?;
    println!("Best renderer: {}", best_renderer.name());

    // Example 3: List GPU-capable renderers
    println!("\n=== GPU-Capable Renderers ===");
    let gpu_renderers = manager.find_factory_by_name("gpu_acceleration");
    if let Some(info) = gpu_renderers {
        println!("- {} (timeout: {}μs)", info.name, info.timeout_microseconds);
    }

    Ok(())
}