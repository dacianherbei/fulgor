//! Demonstrates architecture-aware memory pool configuration and usage.

use forge::memory::pool::{MemoryPool, AlignmentStrategy};
use forge::{ForgeConfig, TargetArchitecture};

fn main() -> anyhow::Result<()> {
    println!("🏗️  Architecture-Aware Memory Pool Demo");
    println!("========================================");

    // Detect current architecture and create optimized configuration
    let current_arch = TargetArchitecture::current();
    println!("Current architecture: {:?}", current_arch);

    // Architecture-specific configuration
    architecture_specific_demo(current_arch)?;

    // Use case specific configurations
    use_case_specific_demo()?;

    // SIMD operations demo
    simd_operations_demo()?;

    // GPU operations demo
    gpu_operations_demo()?;

    Ok(())
}

fn architecture_specific_demo(arch: TargetArchitecture) -> anyhow::Result<()> {
    println!("\n🔧 Architecture-Specific Configuration");
    println!("-------------------------------------");

    let config = ForgeConfig::for_architecture(arch);
    let mut pool = config.create_pool()?;

    println!("Architecture: {:?}", arch);
    println!("Pool size: {} bytes", pool.total_size());
    println!("Base alignment: {} bytes", pool.base_alignment());
    println!("Alignment strategy: {:?}", pool.alignment_strategy());

    // Allocate some data and show alignment
    let ptr = pool.allocate(100, 8)?;
    println!("Allocated at: {:p} (aligned to {} bytes)",
             ptr.as_ptr(), ptr.as_ptr() as usize % pool.base_alignment());

    Ok(())
}

fn use_case_specific_demo() -> anyhow::Result<()> {
    println!("\n⚡ Use Case Specific Configurations");
    println!("----------------------------------");

    // Small allocations - optimized for L1 cache
    let small_config = ForgeConfig::optimized_for_small_allocations();
    let mut small_pool = small_config.create_pool()?;

    println!("Small allocations pool:");
    println!("  Size: {} KB", small_pool.total_size() / 1024);
    println!("  Strategy: {:?}", small_pool.alignment_strategy());

    // Simulate many small allocations
    for i in 0..100 {
        let _ptr = small_pool.allocate(32, 8)?;
        if i == 0 {
            println!("  First allocation aligned to: {} bytes",
                     _ptr.as_ptr() as usize % 64);
        }
    }
    println!("  Allocated 100 small objects efficiently");

    // Large data processing - optimized for virtual memory
    let large_config = ForgeConfig::optimized_for_large_data();
    let mut large_pool = large_config.create_pool()?;

    println!("\nLarge data pool:");
    println!("  Size: {} MB", large_pool.total_size() / (1024 * 1024));
    println!("  Strategy: {:?}", large_pool.alignment_strategy());

    let large_ptr = large_pool.allocate(1024 * 1024, 64)?; // 1MB allocation
    println!("  Large allocation aligned to: {} bytes",
             large_ptr.as_ptr() as usize % 4096);

    Ok(())
}

fn simd_operations_demo() -> anyhow::Result<()> {
    println!("\n🚀 SIMD Operations Demo");
    println!("----------------------");

    // AVX 256-bit operations (32-byte alignment)
    let avx_config = ForgeConfig::optimized_for_simd(32);
    let mut avx_pool = avx_config.create_pool()?;

    println!("AVX 256-bit pool:");
    println!("  Alignment: {} bytes", avx_pool.base_alignment());

    // Allocate vectors for SIMD operations
    let vector_a = avx_pool.allocate(256, 32)?; // 256 bytes = 64 floats
    let vector_b = avx_pool.allocate(256, 32)?;
    let result_vector = avx_pool.allocate(256, 32)?;

    println!("  Vector A: {:p} (32-byte aligned: {})",
             vector_a.as_ptr(),
             vector_a.as_ptr() as usize % 32 == 0);
    println!("  Vector B: {:p} (32-byte aligned: {})",
             vector_b.as_ptr(),
             vector_b.as_ptr() as usize % 32 == 0);
    println!("  Result:   {:p} (32-byte aligned: {})",
             result_vector.as_ptr(),
             result_vector.as_ptr() as usize % 32 == 0);

    // AVX-512 operations (64-byte alignment)
    let avx512_config = ForgeConfig::optimized_for_simd(64);
    let mut avx512_pool = avx512_config.create_pool()?;

    let wide_vector = avx512_pool.allocate(512, 64)?; // 512 bytes = 128 floats
    println!("  AVX-512 vector: {:p} (64-byte aligned: {})",
             wide_vector.as_ptr(),
             wide_vector.as_ptr() as usize % 64 == 0);

    Ok(())
}

fn gpu_operations_demo() -> anyhow::Result<()> {
    println!("\n💎 GPU Operations Demo");
    println!("---------------------");

    let gpu_config = ForgeConfig::optimized_for_gpu();
    let mut gpu_pool = gpu_config.create_pool()?;

    println!("GPU-optimized pool:");
    println!("  Size: {} MB", gpu_pool.total_size() / (1024 * 1024));
    println!("  Alignment: {} bytes", gpu_pool.base_alignment());

    // Allocate GPU transfer buffers
    let vertex_buffer = gpu_pool.allocate(1024 * 1024, 256)?; // 1MB vertex data
    let texture_buffer = gpu_pool.allocate(2 * 1024 * 1024, 256)?; // 2MB texture data
    let uniform_buffer = gpu_pool.allocate(64 * 1024, 256)?; // 64KB uniforms

    println!("  Vertex buffer:  {:p} (256-byte aligned: {})",
             vertex_buffer.as_ptr(),
             vertex_buffer.as_ptr() as usize % 256 == 0);
    println!("  Texture buffer: {:p} (256-byte aligned: {})",
             texture_buffer.as_ptr(),
             texture_buffer.as_ptr() as usize % 256 == 0);
    println!("  Uniform buffer: {:p} (256-byte aligned: {})",
             uniform_buffer.as_ptr(),
             uniform_buffer.as_ptr() as usize % 256 == 0);

    let stats = gpu_pool.get_stats();
    println!("  Memory efficiency: {:.1}%", gpu_pool.efficiency() * 100.0);
    println!("  Total allocations: {}", stats.total_allocations);

    Ok(())
}

fn compare_alignment_strategies() -> anyhow::Result<()> {
    println!("\n📊 Alignment Strategy Comparison");
    println!("-------------------------------");

    let size = 1000;
    let alignment = 8;

    let strategies = [
        ("Minimal", AlignmentStrategy::Minimal),
        ("Cache Line", AlignmentStrategy::CacheLine),
        ("Page Aligned", AlignmentStrategy::Page),
        ("Cache Optimized", AlignmentStrategy::CacheOptimized),
        ("Custom 128", AlignmentStrategy::Custom(128)),
    ];

    for (name, strategy) in strategies {
        let pool = MemoryPool::new_with_strategy(size, alignment, strategy)?;
        println!("  {:<15}: {:>5} bytes (overhead: {:>3} bytes)",
                 name,
                 pool.total_size(),
                 pool.total_size() - size);
    }

    Ok(())
}

// Integration with existing lights renderer precision types
fn integration_with_lights_demo() -> anyhow::Result<()> {
    println!("\n🔗 Integration with Lights Renderer");
    println!("----------------------------------");

    // Simulate DataPrecision from lights crate
    enum DataPrecision {
        F16,
        F32,
        F64,
    }

    let precision = DataPrecision::F32;

    // Choose alignment strategy based on precision and use case
    let (alignment, strategy) = match precision {
        DataPrecision::F16 => (16, AlignmentStrategy::CacheLine), // Compact, cache-friendly
        DataPrecision::F32 => (32, AlignmentStrategy::CacheOptimized), // Balanced
        DataPrecision::F64 => (64, AlignmentStrategy::Custom(64)), // High precision
    };

    let config = ForgeConfig {
        default_pool_size: 4 * 1024 * 1024, // 4MB for renderer
        default_alignment: alignment,
        alignment_strategy: strategy,
        enable_stats: true,
    };

    let mut pool = config.create_pool()?;

    println!("Renderer precision: F32");
    println!("  Pool alignment: {} bytes", pool.base_alignment());
    println!("  Pool strategy: {:?}", pool.alignment_strategy());

    // Allocate renderer buffers
    let splat_buffer = pool.allocate(1024 * 1024, alignment)?; // 1MB splats
    println!("  Splat buffer: {:p} (aligned: {})",
             splat_buffer.as_ptr(),
             splat_buffer.as_ptr() as usize % alignment == 0);

    Ok(())
}
