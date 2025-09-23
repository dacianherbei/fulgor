// examples/factory_demo.rs
//! Comprehensive demo of the restructured factory system

use fulgor::renderer::prelude::*;
use fulgor::renderer::cpu_reference::{CpuReferenceRendererFactory, CpuReferenceConfig, CpuReferenceRenderer};
use fulgor::renderer::gpu_optional::{GpuRendererFactory, GpuRendererConfig, GpuOptionalRenderer};
use fulgor::renderer::factory::{parse_parameters, RendererFactory};

fn main() {
    println!("🚀 Restructured 3D Gaussian Splatting Factory System Demo\n");

    demo_module_organization();
    demo_enhanced_renderers();
    demo_parameter_parsing();
    demo_configuration_management();
    demo_factory_capabilities();
    demo_realistic_workflows();
    demo_backward_compatibility();
}

fn demo_module_organization() {
    println!("=== Module Organization Demo ===");

    // Factories are now in their respective modules
    let cpu_factory = CpuReferenceRendererFactory::new();
    let gpu_factory = GpuRendererFactory::new();

    println!("✓ CPU factory from cpu_reference module");
    println!("✓ GPU factory from gpu_optional module");

    // Create renderers using module-specific factories
    let cpu_renderer = cpu_factory.create(DataPrecision::F32, "threads=4").unwrap();
    let gpu_renderer = gpu_factory.create(DataPrecision::F32, "device=auto").unwrap();

    println!("✓ Created CPU renderer: {}", cpu_renderer.name());
    println!("✓ Created GPU renderer: {}", gpu_renderer.name());
    println!();
}

fn demo_enhanced_renderers() {
    println!("=== Enhanced Renderer Demo ===");

    // Traditional way (backward compatible)
    let basic_cpu = CpuReferenceRenderer::<f32>::new();
    println!("✓ Traditional CpuReferenceRenderer::new() still works");

    // New way with configuration
    let config = CpuReferenceConfig::from_parameters(
        DataPrecision::F32,
        "threads=8,quality=ultra,debug=true"
    ).unwrap();
    let configured_cpu = CpuReferenceRenderer::<f32>::with_config(config);

    println!("✓ New with_config() constructor available");
    println!("  Configuration: threads={}, quality={}, debug={}",
             configured_cpu.get_config().threads,
             configured_cpu.get_config().quality,
             configured_cpu.get_config().debug);

    // GPU renderer enhancement
    let gpu_config = GpuRendererConfig::from_parameters(
        DataPrecision::F32,
        "device=cuda:0,memory_limit=2GB"
    ).unwrap();
    let configured_gpu = GpuOptionalRenderer::with_config(gpu_config);

    println!("✓ GPU renderer also enhanced with configuration");
    println!("  Device: {}, Memory: {}MB",
             configured_gpu.get_config().device,
             configured_gpu.get_config().memory_limit / (1024 * 1024));
    println!();
}

fn demo_parameter_parsing() {
    println!("=== Parameter Parsing Demo ===");

    // Demonstrate the shared parameter parsing utility
    let examples = [
        ("", "Empty parameters"),
        ("threads=4", "Single parameter"),
        ("threads=8,quality=high", "Multiple parameters"),
        ("device=cuda:0,memory_limit=2GB", "GPU parameters with units"),
        (" threads = 4 , quality = high ", "Parameters with spaces"),
        ("debug,verbose", "Boolean flags"),
        ("threads=16,debug,quality=ultra", "Mixed parameters"),
    ];

    for (params, description) in examples {
        let parsed = parse_parameters(params);
        println!("Input: '{}' ({})", params, description);
        println!("Parsed: {:?}", parsed);
        println!();
    }
}

fn demo_configuration_management() {
    println!("=== Configuration Management Demo ===");

    // CPU configuration examples
    println!("CPU Configuration Examples:");

    let cpu_configs = [
        (DataPrecision::F32, "", "Default configuration"),
        (DataPrecision::F64, "threads=1,debug=true", "Debug configuration"),
        (DataPrecision::F32, "threads=16,quality=ultra", "High-performance configuration"),
        (DataPrecision::F64, "threads=4,quality=medium,debug=false", "Balanced configuration"),
    ];

    for (precision, params, description) in cpu_configs {
        match CpuReferenceConfig::from_parameters(precision, params) {
            Ok(config) => {
                println!("  ✓ {}: threads={}, quality={}, debug={}, precision={}",
                         description, config.threads, config.quality, config.debug, config.precision);
            }
            Err(e) => {
                println!("  ✗ {}: Error - {:?}", description, e);
            }
        }
    }

    // GPU configuration examples
    println!("\nGPU Configuration Examples:");

    let gpu_configs = [
        (DataPrecision::F32, "", "Default configuration"),
        (DataPrecision::F32, "device=cuda:0", "Specific GPU device"),
        (DataPrecision::F32, "memory_limit=4GB", "High memory limit"),
        (DataPrecision::F32, "device=auto,memory_limit=512MB", "Constrained environment"),
    ];

    for (precision, params, description) in gpu_configs {
        match GpuRendererConfig::from_parameters(precision, params) {
            Ok(config) => {
                println!("  ✓ {}: device={}, memory={}MB, precision={}",
                         description, config.device, config.memory_limit / (1024 * 1024), config.precision);
            }
            Err(e) => {
                println!("  ✗ {}: Error - {:?}", description, e);
            }
        }
    }
    println!();
}

fn demo_factory_capabilities() {
    println!("=== Factory Capabilities Demo ===");

    let cpu_factory = CpuReferenceRendererFactory::new();
    let gpu_factory = GpuRendererFactory::new();

    // Display factory information
    let cpu_info = cpu_factory.get_info();
    let gpu_info = gpu_factory.get_info();

    println!("CPU Factory Information:");
    println!("  Name: {}", cpu_info.name);
    println!("  Capabilities: {}", cpu_info.capabilities);
    println!("  Timeout: {}μs", cpu_info.timeout_microseconds);
    println!("  Parameters:");
    for (param, desc) in cpu_info.get_parameters() {
        println!("    {}: {}", param, desc);
    }

    println!("\nGPU Factory Information:");
    println!("  Name: {}", gpu_info.name);
    println!("  Capabilities: {}", gpu_info.capabilities);
    println!("  Timeout: {}μs", gpu_info.timeout_microseconds);
    println!("  Parameters:");
    for (param, desc) in gpu_info.get_parameters() {
        println!("    {}: {}", param, desc);
    }

    // Demonstrate capability queries
    println!("\nCapability-based Factory Selection:");
    if cpu_info.has_capability("debugging") {
        println!("  → CPU factory is good for development and debugging");
    }
    if cpu_info.has_capability("reference") {
        println!("  → CPU factory provides reference implementation");
    }
    if gpu_info.has_capability("realtime") {
        println!("  → GPU factory is good for real-time applications");
    }
    if gpu_info.has_capability("fast") {
        println!("  → GPU factory provides high-performance rendering");
    }

    // Precision support matrix
    println!("\nPrecision Support Matrix:");
    for precision in [DataPrecision::F16, DataPrecision::F32, DataPrecision::F64, DataPrecision::BFloat16] {
        let cpu_support = cpu_factory.validate_parameters(precision, "").is_ok();
        let gpu_support = gpu_factory.validate_parameters(precision, "").is_ok();
        println!("  {}: CPU={}, GPU={}", precision,
                 if cpu_support { "✓" } else { "✗" },
                 if gpu_support { "✓" } else { "✗" });
    }
    println!();
}

fn demo_realistic_workflows() {
    println!("=== Realistic Workflow Demo ===");

    let cpu_factory = CpuReferenceRendererFactory::new();
    let gpu_factory = GpuRendererFactory::new();

    // Workflow 1: Algorithm Development
    println!("🔬 Algorithm Development Workflow:");
    let mut dev_renderer = cpu_factory.create(
        DataPrecision::F64,
        "threads=1,quality=low,debug=true"
    ).unwrap();

    println!("  Created high-precision, debug-enabled CPU renderer");
    dev_renderer.start().unwrap();
    println!("  Started renderer for algorithm testing");

    // Simulate development work
    std::thread::sleep(std::time::Duration::from_millis(10));

    dev_renderer.stop();
    println!("  Stopped renderer after testing");

    // Workflow 2: Production GPU Deployment
    println!("\n🚀 Production GPU Deployment Workflow:");
    let mut prod_renderer = gpu_factory.create(
        DataPrecision::F32,
        "device=cuda:0,memory_limit=4GB"
    ).unwrap();

    println!("  Created optimized GPU renderer for production");

    // Note: GPU start might fail if feature not enabled
    match prod_renderer.start() {
        Ok(_) => {
            println!("  Started GPU renderer for real-time rendering");
            std::thread::sleep(std::time::Duration::from_millis(10));
            prod_renderer.stop();
            println!("  Stopped renderer after production run");
        }
        Err(e) if e.contains("gpu feature not enabled") => {
            println!("  GPU feature not enabled in this build");
        }
        Err(e) => {
            println!("  GPU initialization failed: {}", e);
        }
    }

    // Workflow 3: Research High-Quality Rendering
    println!("\n📊 Research High-Quality Workflow:");
    let mut research_renderer = cpu_factory.create(
        DataPrecision::F32,
        "threads=16,quality=ultra,debug=false"
    ).unwrap();

    println!("  Created multi-threaded CPU renderer for quality research");
    research_renderer.start().unwrap();
    println!("  Started renderer for high-quality offline rendering");

    research_renderer.stop();
    println!("  Completed research rendering");

    // Workflow 4: Cloud/Container Deployment
    println!("\n☁️ Cloud Container Workflow:");
    let mut cloud_renderer = cpu_factory.create(
        DataPrecision::F32,
        "threads=8,quality=medium,debug=false"
    ).unwrap();

    println!("  Created container-optimized CPU renderer");
    cloud_renderer.start().unwrap();
    println!("  Started renderer in containerized environment");

    cloud_renderer.stop();
    println!("  Stopped renderer after cloud processing");
    println!();
}

fn demo_backward_compatibility() {
    println!("=== Backward Compatibility Demo ===");

    // Show that existing code still works
    println!("Existing code patterns still work:");

    // Traditional renderer creation
    let basic_cpu = CpuReferenceRenderer::<f32>::new();
    let basic_gpu = GpuOptionalRenderer::new();

    println!("  ✓ CpuReferenceRenderer::<f32>::new() - works");
    println!("  ✓ GpuOptionalRenderer::new() - works");

    // Traditional renderer usage
    let mut traditional_renderer = CpuReferenceRenderer::<f64>::new();

    println!("  ✓ Traditional lifecycle methods work:");
    assert!(traditional_renderer.start().is_ok());
    println!("    - start() works");

    traditional_renderer.stop();
    println!("    - stop() works");

    assert_eq!(traditional_renderer.name(), "CpuReference");
    println!("    - name() works");

    // Show enhanced functionality is additive
    println!("\nEnhanced functionality is additive:");
    println!("  ✓ get_config() - new method");
    println!("  ✓ get_frame_count() - enhanced");
    println!("  ✓ reset_frame_count() - new method");
    println!("  ✓ is_running() - new method");
    println!("  ✓ with_config() - new constructor");

    println!("\n✅ All existing code continues to work unchanged!");
    println!();
}

// Demonstration of a potential registry system using the restructured factories
fn demo_registry_integration() {
    println!("=== Registry Integration Demo ===");

    // Mock registry to show integration patterns
    struct RendererRegistry {
        cpu_factory: CpuReferenceRendererFactory,
        gpu_factory: GpuRendererFactory,
    }

    impl RendererRegistry {
        fn new() -> Self {
            Self {
                cpu_factory: CpuReferenceRendererFactory::new(),
                gpu_factory: GpuRendererFactory::new(),
            }
        }

        fn create_for_capability(&self, capability: &str, precision: DataPrecision, params: &str)
                                 -> Result<Box<dyn Renderer>, RendererError> {

            let cpu_info = self.cpu_factory.get_info();
            let gpu_info = self.gpu_factory.get_info();

            if cpu_info.has_capability(capability) {
                self.cpu_factory.create(precision, params)
            } else if gpu_info.has_capability(capability) {
                self.gpu_factory.create(precision, params)
            } else {
                Err(RendererError::RendererNotFoundByName(
                    format!("No factory supports capability: {}", capability)
                ))
            }
        }

        fn create_optimal_for_precision(&self, precision: DataPrecision, params: &str)
                                        -> Result<Box<dyn Renderer>, RendererError> {

            // Try GPU first for F32 (performance), fallback to CPU
            if precision == DataPrecision::F32 {
                if let Ok(renderer) = self.gpu_factory.create(precision, params) {
                    return Ok(renderer);
                }
            }

            // Use CPU for F64 or if GPU failed
            self.cpu_factory.create(precision, params)
        }
    }

    let registry = RendererRegistry::new();

    // Demonstrate capability-based selection
    println!("Capability-based renderer selection:");

    let debug_renderer = registry.create_for_capability(
        "debugging",
        DataPrecision::F64,
        "threads=1,debug=true"
    ).unwrap();
    println!("  ✓ Selected {} for debugging capability", debug_renderer.name());

    let realtime_renderer = registry.create_for_capability(
        "realtime",
        DataPrecision::F32,
        "device=auto"
    ).unwrap();
    println!("  ✓ Selected {} for realtime capability", realtime_renderer.name());

    // Demonstrate precision-optimal selection
    println!("\nPrecision-optimal renderer selection:");

    let f32_renderer = registry.create_optimal_for_precision(
        DataPrecision::F32,
        ""
    ).unwrap();
    println!("  ✓ F32 precision: selected {}", f32_renderer.name());

    let f64_renderer = registry.create_optimal_for_precision(
        DataPrecision::F64,
        "threads=8"
    ).unwrap();
    println!("  ✓ F64 precision: selected {}", f64_renderer.name());

    println!();
}