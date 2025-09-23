// tests/integration_tests.rs
//! Integration tests for the restructured factory system

use fulgor::renderer::prelude::*;
use fulgor::renderer::{ReferenceRendererFactory, ReferenceRendererConfig};
use fulgor::renderer::factory::{GpuRendererFactory, GpuRendererConfig};
use fulgor::renderer::factory::{parse_parameters, RendererFactory};
use std::time::Duration;

#[test]
fn test_restructured_development_workflow() {
    println!("=== Restructured Development Workflow Test ===");

    // Developer starts with CPU reference for algorithm development
    let cpu_factory = CpuReferenceRendererFactory::new();

    // Debug mode with single thread for step-through debugging
    let mut dev_renderer = cpu_factory.create(
        DataPrecision::F64,
        "threads=1,quality=low,debug=true"
    ).unwrap();

    println!("Created development renderer: {}", dev_renderer.name());

    // Test development cycle
    assert!(dev_renderer.start().is_ok());

    // Simulate development work
    std::thread::sleep(Duration::from_millis(10));

    dev_renderer.stop();

    // Move to production GPU rendering
    let gpu_factory = GpuRendererFactory::new();
    let mut prod_renderer = gpu_factory.create(
        DataPrecision::F32,
        "device=cuda:0,memory_limit=2GB"
    ).unwrap();

    println!("Created production renderer: {}", prod_renderer.name());

    // Note: GPU renderer start might fail if GPU feature not enabled
    match prod_renderer.start() {
        Ok(_) => {
            println!("GPU renderer started successfully");
            prod_renderer.stop();
        }
        Err(e) if e.contains("gpu feature not enabled") => {
            println!("GPU feature not enabled, skipping GPU test");
        }
        Err(e) => {
            println!("GPU initialization failed: {}", e);
        }
    }
}

#[test]
fn test_enhanced_renderer_functionality() {
    println!("=== Enhanced Renderer Functionality Test ===");

    let cpu_factory = CpuReferenceRendererFactory::new();

    // Test F32 renderer with configuration
    let mut f32_renderer = cpu_factory.create(
        DataPrecision::F32,
        "threads=4,quality=high,debug=true"
    ).unwrap();

    assert_eq!(f32_renderer.name(), "CpuReference");
    assert!(f32_renderer.start().is_ok());

    // Test F64 renderer with different configuration
    let mut f64_renderer = cpu_factory.create(
        DataPrecision::F64,
        "threads=2,quality=ultra,debug=false"
    ).unwrap();

    assert_eq!(f64_renderer.name(), "CpuReference");
    assert!(f64_renderer.start().is_ok());

    f32_renderer.stop();
    f64_renderer.stop();
    println!("Enhanced renderer functionality: OK");
}

#[test]
fn test_parameter_parsing_utility() {
    println!("=== Parameter Parsing Utility Test ===");

    // Test comprehensive parameter parsing
    let test_cases = [
        ("", HashMap::new()),
        ("debug", {
            let mut map = HashMap::new();
            map.insert("debug".to_string(), "true".to_string());
            map
        }),
        ("threads=8,quality=high", {
            let mut map = HashMap::new();
            map.insert("threads".to_string(), "8".to_string());
            map.insert("quality".to_string(), "high".to_string());
            map
        }),
        (" device = cuda:0 , memory_limit = 2GB ", {
            let mut map = HashMap::new();
            map.insert("device".to_string(), "cuda:0".to_string());
            map.insert("memory_limit".to_string(), "2GB".to_string());
            map
        }),
    ];

    for (input, expected) in test_cases {
        let parsed = parse_parameters(input);
        assert_eq!(parsed, expected, "Failed for input: '{}'", input);
    }

    println!("Parameter parsing utility: OK");
}

#[test]
fn test_cpu_config_validation() {
    println!("=== CPU Configuration Validation Test ===");

    // Test valid configurations
    let valid_configs = [
        (DataPrecision::F32, ""),
        (DataPrecision::F64, "threads=1"),
        (DataPrecision::F32, "threads=16,quality=ultra,debug=true"),
        (DataPrecision::F64, "quality=low,debug=false"),
    ];

    for (precision, params) in valid_configs {
        let config = CpuReferenceConfig::from_parameters(precision, params);
        assert!(config.is_ok(), "Failed for precision {:?}, params: '{}'", precision, params);
    }

    // Test invalid configurations
    let invalid_configs = [
        (DataPrecision::F32, "threads=0"),
        (DataPrecision::F64, "quality=invalid"),
        (DataPrecision::F32, "threads=abc"),
        (DataPrecision::F64, "unsupported_param=value"),
        (DataPrecision::F32, "debug=maybe"),
    ];

    for (precision, params) in invalid_configs {
        let config = CpuReferenceConfig::from_parameters(precision, params);
        assert!(config.is_err(), "Should have failed for precision {:?}, params: '{}'", precision, params);
    }

    println!("CPU configuration validation: OK");
}

#[test]
fn test_gpu_config_validation() {
    println!("=== GPU Configuration Validation Test ===");

    // Test valid configurations
    let valid_configs = [
        (DataPrecision::F32, ""),
        (DataPrecision::F32, "device=auto"),
        (DataPrecision::F32, "device=cuda:0,memory_limit=1GB"),
        (DataPrecision::F32, "memory_limit=512MB"),
        (DataPrecision::F32, "device=cuda:5,memory_limit=4GB"),
    ];

    for (precision, params) in valid_configs {
        let config = GpuRendererConfig::from_parameters(precision, params);
        assert!(config.is_ok(), "Failed for precision {:?}, params: '{}'", precision, params);
    }

    // Test invalid configurations
    let invalid_configs = [
        (DataPrecision::F64, ""), // GPU only supports F32
        (DataPrecision::F16, ""),
        (DataPrecision::F32, "device=invalid"),
        (DataPrecision::F32, "memory_limit=32MB"), // Too small
        (DataPrecision::F32, "memory_limit=invalid"),
        (DataPrecision::F32, "unsupported_param=value"),
    ];

    for (precision, params) in invalid_configs {
        let config = GpuRendererConfig::from_parameters(precision, params);
        assert!(config.is_err(), "Should have failed for precision {:?}, params: '{}'", precision, params);
    }

    println!("GPU configuration validation: OK");
}

#[test]
fn test_factory_information_consistency() {
    println!("=== Factory Information Consistency Test ===");

    let cpu_factory = CpuReferenceRendererFactory::new();
    let gpu_factory = GpuRendererFactory::new();

    // Test CPU factory info
    let cpu_info = cpu_factory.get_info();
    println!("CPU Factory: {}", cpu_info.name);
    println!("CPU Capabilities: {}", cpu_info.capabilities);
    println!("CPU Timeout: {}μs", cpu_info.timeout_microseconds);

    assert_eq!(cpu_info.timeout_microseconds, 1000);
    assert!(cpu_info.has_capability("software"));
    assert!(cpu_info.has_capability("reference"));
    assert!(cpu_info.has_capability("debugging"));
    assert!(cpu_info.has_capability("cpu"));
    assert!(!cpu_info.has_capability("gpu"));

    // Test GPU factory info
    let gpu_info = gpu_factory.get_info();
    println!("GPU Factory: {}", gpu_info.name);
    println!("GPU Capabilities: {}", gpu_info.capabilities);
    println!("GPU Timeout: {}μs", gpu_info.timeout_microseconds);

    assert_eq!(gpu_info.timeout_microseconds, 50000);
    assert!(gpu_info.has_capability("gpu"));
    assert!(gpu_info.has_capability("cuda"));
    assert!(gpu_info.has_capability("fast"));
    assert!(gpu_info.has_capability("realtime"));
    assert!(!gpu_info.has_capability("cpu"));

    // Test parameter information
    assert!(cpu_info.has_parameter("threads"));
    assert!(cpu_info.has_parameter("quality"));
    assert!(cpu_info.has_parameter("debug"));

    assert!(gpu_info.has_parameter("device"));
    assert!(gpu_info.has_parameter("memory_limit"));

    println!("Factory information consistency: OK");
}

#[test]
fn test_precision_support_matrix() {
    println!("=== Precision Support Matrix Test ===");

    let cpu_factory = CpuReferenceRendererFactory::new();
    let gpu_factory = GpuRendererFactory::new();

    println!("CPU Factory Precision Support:");
    for precision in [DataPrecision::F16, DataPrecision::F32, DataPrecision::F64, DataPrecision::BFloat16] {
        let result = cpu_factory.create(precision, "threads=1");
        match result {
            Ok(_) => println!("  ✓ {} supported", precision),
            Err(_) => println!("  ✗ {} not supported", precision),
        }
    }

    println!("\nGPU Factory Precision Support:");
    for precision in [DataPrecision::F16, DataPrecision::F32, DataPrecision::F64, DataPrecision::BFloat16] {
        let result = gpu_factory.create(precision, "device=auto");
        match result {
            Ok(_) => println!("  ✓ {} supported", precision),
            Err(_) => println!("  ✗ {} not supported", precision),
        }
    }

    // Validate expected support
    assert!(cpu_factory.create(DataPrecision::F32, "").is_ok());
    assert!(cpu_factory.create(DataPrecision::F64, "").is_ok());
    assert!(cpu_factory.create(DataPrecision::F16, "").is_err());

    assert!(gpu_factory.create(DataPrecision::F32, "").is_ok());
    assert!(gpu_factory.create(DataPrecision::F64, "").is_err());

    println!("Precision support matrix: OK");
}

#[test]
fn test_validation_vs_creation_consistency() {
    println!("=== Validation vs Creation Consistency Test ===");

    let cpu_factory = CpuReferenceRendererFactory::new();
    let gpu_factory = GpuRendererFactory::new();

    // Test cases where validation and creation should agree
    let test_cases = [
        (&cpu_factory as &dyn RendererFactory, DataPrecision::F32, "threads=4,quality=high"),
        (&cpu_factory as &dyn RendererFactory, DataPrecision::F64, "debug=true"),
        (&cpu_factory as &dyn RendererFactory, DataPrecision::F16, ""), // Should fail
        (&gpu_factory as &dyn RendererFactory, DataPrecision::F32, "device=cuda:0"),
        (&gpu_factory as &dyn RendererFactory, DataPrecision::F64, ""), // Should fail
    ];

    for (factory, precision, params) in test_cases {
        let validation_result = factory.validate_parameters(precision, params);
        let creation_result = factory.create(precision, params);

        assert_eq!(
            validation_result.is_ok(),
            creation_result.is_ok(),
            "Validation and creation disagree for precision {:?}, params: '{}'",
            precision, params
        );
    }

    println!("Validation vs creation consistency: OK");
}

#[test]
fn test_realistic_usage_scenarios() {
    println!("=== Realistic Usage Scenarios Test ===");

    let cpu_factory = CpuReferenceRendererFactory::new();
    let gpu_factory = GpuRendererFactory::new();

    // Scenario 1: Research/Development Environment
    println!("Scenario 1: Research Environment");
    let research_renderer = cpu_factory.create(
        DataPrecision::F64,  // High precision for research
        "threads=1,quality=low,debug=true"  // Single thread for debugging
    );
    assert!(research_renderer.is_ok());

    // Scenario 2: Production CPU Rendering Farm
    println!("Scenario 2: Production CPU Farm");
    let cpu_farm_renderer = cpu_factory.create(
        DataPrecision::F32,  // Balanced precision/performance
        "threads=32,quality=ultra,debug=false"  // Many threads, high quality
    );
    assert!(cpu_farm_renderer.is_ok());

    // Scenario 3: Real-time Interactive Application
    println!("Scenario 3: Real-time Interactive");
    let interactive_renderer = gpu_factory.create(
        DataPrecision::F32,
        "device=cuda:0,memory_limit=4GB"  // Specific GPU with plenty of memory
    );
    assert!(interactive_renderer.is_ok());

    // Scenario 4: Embedded/Constrained Environment
    println!("Scenario 4: Embedded System");
    let embedded_renderer = gpu_factory.create(
        DataPrecision::F32,
        "device=auto,memory_limit=256MB"  // Auto-detect with memory constraints
    );
    assert!(embedded_renderer.is_ok());

    // Scenario 5: Cloud/Container Environment
    println!("Scenario 5: Cloud Container");
    let cloud_cpu_renderer = cpu_factory.create(
        DataPrecision::F32,
        "threads=8,quality=medium,debug=false"  // Fixed thread count for containers
    );
    assert!(cloud_cpu_renderer.is_ok());

    println!("All realistic scenarios: OK");
}

#[test]
fn test_concurrent_factory_usage() {
    println!("=== Concurrent Factory Usage Test ===");

    use std::sync::Arc;
    use std::thread;

    let cpu_factory = Arc::new(CpuReferenceRendererFactory::new());
    let gpu_factory = Arc::new(GpuRendererFactory::new());

    let mut handles = vec![];

    // Create multiple renderers concurrently
    for i in 0..4 {
        let cpu_factory_clone = Arc::clone(&cpu_factory);
        let gpu_factory_clone = Arc::clone(&gpu_factory);

        let handle = thread::spawn(move || {
            // Create CPU renderer
            let cpu_params = format!("threads={},quality=medium,debug=false", i + 1);
            let cpu_renderer = cpu_factory_clone.create(DataPrecision::F32, &cpu_params);
            assert!(cpu_renderer.is_ok());

            // Create GPU renderer
            let gpu_params = format!("device=auto,memory_limit={}GB", i + 1);
            let gpu_renderer = gpu_factory_clone.create(DataPrecision::F32, &gpu_params);
            assert!(gpu_renderer.is_ok());

            println!("Thread {} created renderers successfully", i);
        });

        handles.push(handle);
    }

    // Wait for all threads to complete
    for handle in handles {
        handle.join().unwrap();
    }

    println!("Concurrent factory usage: OK");
}

#[test]
fn test_memory_limit_parsing_edge_cases() {
    println!("=== Memory Limit Parsing Edge Cases Test ===");

    // Test various memory limit formats
    let test_cases = [
        ("memory_limit=1GB", 1024 * 1024 * 1024),
        ("memory_limit=512MB", 512 * 1024 * 1024),
        ("memory_limit=1073741824", 1073741824), // Raw bytes
        ("memory_limit=2GB", 2 * 1024 * 1024 * 1024),
        ("memory_limit=256MB", 256 * 1024 * 1024),
    ];

    for (params, expected_bytes) in test_cases {
        let config = GpuRendererConfig::from_parameters(DataPrecision::F32, params).unwrap();
        assert_eq!(config.memory_limit, expected_bytes, "Failed for params: '{}'", params);
    }

    // Test invalid memory formats
    let invalid_cases = [
        "memory_limit=32MB", // Too small
        "memory_limit=invalid",
        "memory_limit=1TB", // Unsupported unit
        "memory_limit=-1GB",
    ];

    for params in invalid_cases {
        let config = GpuRendererConfig::from_parameters(DataPrecision::F32, params);
        assert!(config.is_err(), "Should have failed for params: '{}'", params);
    }

    println!("Memory limit parsing edge cases: OK");
}

#[test]
fn test_factory_error_messages() {
    println!("=== Factory Error Messages Test ===");

    let cpu_factory = CpuReferenceRendererFactory::new();
    let gpu_factory = GpuRendererFactory::new();

    // Test descriptive error messages
    match cpu_factory.create(DataPrecision::F16, "") {
        Err(RendererError::UnsupportedPrecision(precision)) => {
            assert_eq!(precision, DataPrecision::F16);
            println!("✓ CPU factory correctly rejects F16 precision");
        }
        _ => panic!("Expected UnsupportedPrecision error"),
    }

    match cpu_factory.create(DataPrecision::F32, "threads=0") {
        Err(RendererError::InvalidParameters(msg)) => {
            assert!(msg.contains("threads must be greater than 0"));
            println!("✓ CPU factory provides descriptive parameter error");
        }
        _ => panic!("Expected InvalidParameters error"),
    }

    match gpu_factory.create(DataPrecision::F64, "") {
        Err(RendererError::UnsupportedPrecision(precision)) => {
            assert_eq!(precision, DataPrecision::F64);
            println!("✓ GPU factory correctly rejects F64 precision");
        }
        _ => panic!("Expected UnsupportedPrecision error"),
    }

    match gpu_factory.create(DataPrecision::F32, "memory_limit=32MB") {
        Err(RendererError::InvalidParameters(msg)) => {
            assert!(msg.contains("memory_limit must be at least 64MB"));
            println!("✓ GPU factory provides descriptive memory limit error");
        }
        _ => panic!("Expected InvalidParameters error"),
    }

    println!("Factory error messages: OK");
}

#[test]
fn test_default_configurations() {
    println!("=== Default Configurations Test ===");

    // Test CPU default configuration
    let cpu_config = CpuReferenceConfig::default();
    assert_eq!(cpu_config.quality, "medium");
    assert_eq!(cpu_config.debug, false);
    assert_eq!(cpu_config.precision, DataPrecision::F32);
    assert!(cpu_config.threads > 0);
    println!("✓ CPU default configuration is sensible");

    // Test GPU default configuration
    let gpu_config = GpuRendererConfig::default();
    assert_eq!(gpu_config.device, "auto");
    assert_eq!(gpu_config.memory_limit, 1024 * 1024 * 1024); // 1GB
    assert_eq!(gpu_config.precision, DataPrecision::F32);
    println!("✓ GPU default configuration is sensible");

    // Test that factories create renderers with defaults when no params provided
    let cpu_factory = CpuReferenceRendererFactory::new();
    let gpu_factory = GpuRendererFactory::new();

    let cpu_renderer = cpu_factory.create(DataPrecision::F32, "").unwrap();
    let gpu_renderer = gpu_factory.create(DataPrecision::F32, "").unwrap();

    assert_eq!(cpu_renderer.name(), "CpuReference");
    assert_eq!(gpu_renderer.name(), "GpuOptional");

    println!("Default configurations: OK");
}

#[cfg(test)]
mod manager_error_integration_tests {
    use super::*;
    use crate::renderer::factory::MockRendererFactory;

    #[test]
    fn test_renderer_not_found_by_name_error() {
        let manager = RendererManager::new();

        // Try to create a renderer with a non-existent name
        let result = manager.create_by_name("NonExistentRenderer", DataPrecision::F32, "");

        assert!(result.is_err());
        match result.unwrap_err() {
            RendererError::RendererNotFoundByName(name) => {
                assert_eq!(name, "NonExistentRenderer");
            }
            other => panic!("Expected RendererNotFoundByName error, got: {:?}", other),
        }
    }

    #[test]
    fn test_renderer_not_found_by_name_error_display() {
        let error = RendererError::RendererNotFoundByName("TestRenderer".to_string());
        let error_message = format!("{}", error);

        assert_eq!(error_message, "Renderer factory not found with name: TestRenderer");
    }

    #[test]
    fn test_successful_renderer_creation_by_name() {
        // This test would work once you have the factory registration implemented
        // For now, it's just a placeholder to show the intended usage

        let mut manager = RendererManager::new();

        // Register a mock factory (this would need the factory registration system)
        // let factory = Box::new(MockRendererFactory::new("TestRenderer"));
        // manager.register(factory).unwrap();

        // Then try to create by name:
        // let result = manager.create_by_name("TestRenderer", DataPrecision::F32, "");
        // assert!(result.is_ok());
    }

    #[test]
    fn test_validate_parameters_for_missing_factory() {
        let manager = RendererManager::new();

        let result = manager.validate_parameters_for("MissingFactory", DataPrecision::F32, "test");

        assert!(result.is_err());
        match result.unwrap_err() {
            RendererError::RendererNotFoundByName(name) => {
                assert_eq!(name, "MissingFactory");
            }
            other => panic!("Expected RendererNotFoundByName error, got: {:?}", other),
        }
    }
}
