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
        let config = OpenGL3RendererConfig::from_parameters(precision, params);
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
        let config = OpenGL3RendererConfig::from_parameters(precision, params);
        assert!(config.is_err(), "Should have failed for precision {:?}, params: '{}'", precision, params);
    }

    println!("GPU configuration validation: OK");
}

#[test]
fn test_enhanced_renderer_functionality() {
    println!("=== Enhanced Renderer Functionality Test ===");

    let cpu_factory = ReferenceRendererFactory::new();

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