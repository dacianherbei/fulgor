// File: crates/forge/tests/integration_tests.rs
//! Integration tests for the forge memory management system.
//!
//! These tests validate the complete memory pool system including architecture-aware
//! alignment strategies, configuration management, and realistic usage scenarios.

use forge::memory::pool::{MemoryPool, PoolError, AlignmentStrategy};
use forge::{ForgeConfig, TargetArchitecture};
use std::thread;
use std::time::Duration;

#[test]
fn test_memory_pool_creation_and_basic_usage() {
    let mut pool = MemoryPool::new(1024).unwrap();

    // Test basic properties
    assert!(pool.total_size() >= 1024); // May be aligned up
    assert_eq!(pool.current_offset(), 0);
    assert!(pool.is_empty());
    assert_eq!(pool.efficiency(), 1.0); // No allocations yet
    assert_eq!(pool.fragmentation_ratio(), 0.0); // Pools have zero fragmentation

    // Test basic allocation
    let ptr = pool.allocate(64, 8).unwrap();
    assert!(!ptr.as_ptr().is_null());
    assert_eq!(pool.current_offset(), 64);
    assert!(!pool.is_empty());

    // Verify default alignment strategy
    assert_eq!(pool.alignment_strategy(), AlignmentStrategy::CacheOptimized);
}

#[test]
fn test_alignment_requirements() {
    let mut pool = MemoryPool::new(1024).unwrap();

    let test_cases = [
        (64, 1),   // 1-byte alignment
        (64, 2),   // 2-byte alignment
        (64, 4),   // 4-byte alignment
        (64, 8),   // 8-byte alignment
        (64, 16),  // 16-byte alignment
        (64, 32),  // 32-byte alignment
        (64, 64),  // 64-byte alignment
    ];

    for (size, align) in test_cases {
        pool.reset();
        let ptr = pool.allocate(size, align).unwrap();

        // With CacheOptimized strategy, alignment will be at least 64 bytes
        let effective_align = 64.max(align);
        assert_eq!(ptr.as_ptr() as usize % effective_align, 0,
                   "Failed alignment requirement: {} bytes (effective: {})", align, effective_align);
    }
}

#[test]
fn test_alignment_strategies() {
    let size = 1000;
    let base_align = 8;

    // Test each alignment strategy
    let strategies = [
        ("Minimal", AlignmentStrategy::Minimal, 1000),
        ("CacheLine", AlignmentStrategy::CacheLine, 1024), // Aligned to 64 bytes
        ("Page", AlignmentStrategy::Page, 4096), // Aligned to 4KB
        ("CacheOptimized", AlignmentStrategy::CacheOptimized, 1024), // Aligned to 64 bytes
        ("Custom128", AlignmentStrategy::Custom(128), 1024), // Aligned to 128 bytes
    ];

    for (name, strategy, expected_size) in strategies {
        let pool = MemoryPool::new_with_strategy(size, base_align, strategy).unwrap();
        assert_eq!(pool.total_size(), expected_size,
                   "Strategy {} should result in {} bytes", name, expected_size);
        assert_eq!(pool.alignment_strategy(), strategy);
        assert_eq!(pool.base_alignment(), base_align);
    }
}

#[test]
fn test_simd_alignment_strategies() {
    // Test SIMD-specific alignments
    let simd_tests = [
        (16, "SSE/NEON"),    // 128-bit vectors
        (32, "AVX"),         // 256-bit vectors
        (64, "AVX-512"),     // 512-bit vectors
    ];

    for (vector_width, description) in simd_tests {
        let mut pool = MemoryPool::new_with_strategy(
            1024 * 1024, vector_width, AlignmentStrategy::Custom(vector_width)
        ).unwrap();

        // Allocate SIMD data
        let simd_ptr = pool.allocate(vector_width * 8, vector_width).unwrap(); // 8 vectors
        assert_eq!(simd_ptr.as_ptr() as usize % vector_width, 0,
                   "{} vectors should be {}-byte aligned", description, vector_width);

        assert_eq!(pool.base_alignment(), vector_width);
        assert_eq!(pool.alignment_strategy(), AlignmentStrategy::Custom(vector_width));
    }
}

#[test]
fn test_pool_exhaustion() {
    let mut pool = MemoryPool::new_with_strategy(200, 8, AlignmentStrategy::Minimal).unwrap();

    // Get the actual pool size (should be exactly 200 with minimal strategy)
    let actual_size = pool.total_size();
    assert_eq!(actual_size, 200);

    // Fill most of the pool
    let large_allocation = actual_size - 64; // Leave small amount
    let _ptr1 = pool.allocate(large_allocation, 8).unwrap();

    // This should fail - trying to allocate more than remaining space
    let result = pool.allocate(128, 8); // More than the 64 bytes left
    assert!(matches!(result, Err(PoolError::PoolExhausted { .. })));

    // Verify error details
    if let Err(PoolError::PoolExhausted { requested, available }) = result {
        assert_eq!(requested, 128);
        assert_eq!(available, 64);
    }
}

#[test]
fn test_invalid_alignment() {
    let mut pool = MemoryPool::new(1024).unwrap();

    // Test invalid alignments (not powers of 2)
    let invalid_alignments = [3, 5, 6, 7, 9, 10, 15];

    for align in invalid_alignments {
        let result = pool.allocate(64, align);
        assert!(matches!(result, Err(PoolError::InvalidAlignment { .. })),
                "Should fail for alignment: {}", align);
    }

    // Test invalid alignment in pool creation
    let result = MemoryPool::new_with_alignment(1024, 3);
    assert!(matches!(result, Err(PoolError::InvalidAlignment { .. })));
}

#[test]
fn test_reset_functionality() {
    let mut pool = MemoryPool::new(1024).unwrap();

    // Allocate some memory
    let _ptr1 = pool.allocate(100, 8).unwrap();
    let checkpoint = pool.current_offset();

    let _ptr2 = pool.allocate(200, 8).unwrap();
    // With cache-optimized strategy, alignment may affect the actual offset
    let expected_offset = checkpoint + 200;
    assert!(pool.current_offset() >= expected_offset);

    // Reset to checkpoint
    pool.reset_to(checkpoint).unwrap();
    assert_eq!(pool.current_offset(), checkpoint);

    // Reset completely
    pool.reset();
    assert_eq!(pool.current_offset(), 0);
    assert!(pool.is_empty());
}

#[test]
fn test_high_water_mark() {
    let mut pool = MemoryPool::new(1024).unwrap();

    let _ptr1 = pool.allocate(100, 8).unwrap();
    let hwm1 = pool.high_water_mark();
    assert!(hwm1 >= 100); // May be larger due to alignment

    let _ptr2 = pool.allocate(200, 8).unwrap();
    let hwm2 = pool.high_water_mark();
    assert!(hwm2 >= hwm1 + 200); // Should increase

    // Reset and allocate smaller amount
    pool.reset();
    let _ptr3 = pool.allocate(50, 8).unwrap();
    assert_eq!(pool.high_water_mark(), hwm2); // Should remain at peak
}

#[test]
fn test_high_water_mark_aligned() {
    // Use minimal strategy to get predictable results
    let mut pool = MemoryPool::new_with_strategy(1024, 8, AlignmentStrategy::Minimal).unwrap();

    // Use sizes that are multiples of 8 to avoid alignment padding
    let _ptr1 = pool.allocate(96, 8).unwrap();  // 96 is multiple of 8
    assert_eq!(pool.high_water_mark(), 96);

    let _ptr2 = pool.allocate(200, 8).unwrap(); // 200 is multiple of 8
    assert_eq!(pool.high_water_mark(), 296);   // 96 + 200 = 296

    // Reset and allocate smaller amount
    pool.reset();
    let _ptr3 = pool.allocate(48, 8).unwrap();  // 48 is multiple of 8
    assert_eq!(pool.high_water_mark(), 296);   // Should remain at peak
}

#[test]
fn test_alignment_behavior() {
    let mut pool = MemoryPool::new_with_strategy(1024, 8, AlignmentStrategy::Minimal).unwrap();

    // Allocate 100 bytes with 8-byte alignment
    let _ptr1 = pool.allocate(100, 8).unwrap();
    let offset_after_first = pool.current_offset();
    assert_eq!(offset_after_first, 100);

    // Next allocation should be aligned to 8-byte boundary
    let _ptr2 = pool.allocate(200, 8).unwrap();
    let offset_after_second = pool.current_offset();

    // 100 rounded up to next 8-byte boundary is 104
    // Then add 200 bytes: 104 + 200 = 304
    assert_eq!(offset_after_second, 304);

    // Verify alignment
    let ptr2_addr = _ptr2.as_ptr() as usize;
    assert_eq!(ptr2_addr % 8, 0, "Second allocation should be 8-byte aligned");
}

#[test]
fn test_can_allocate() {
    let pool = MemoryPool::new_with_strategy(1000, 8, AlignmentStrategy::Minimal).unwrap();

    // Test various allocation sizes
    assert!(pool.can_allocate(500, 8));
    assert!(pool.can_allocate(1000, 8));
    assert!(!pool.can_allocate(1001, 8));

    // Test invalid alignment
    assert!(!pool.can_allocate(100, 3)); // Invalid alignment

    // Test with different alignments
    assert!(pool.can_allocate(100, 16));
    assert!(pool.can_allocate(100, 32));
}

#[test]
fn test_statistics_tracking() {
    let mut pool = MemoryPool::new(1024).unwrap();

    pool.allocate(64, 8).unwrap();
    pool.allocate(128, 16).unwrap();
    pool.reset();

    let stats = pool.get_stats();
    assert_eq!(stats.total_allocations, 2);
    assert_eq!(stats.total_bytes_allocated, 192);
    assert_eq!(stats.reset_count, 1);
    assert_eq!(stats.largest_allocation, 128);
    assert_eq!(stats.smallest_allocation, 64);
    assert_eq!(stats.average_allocation_size, 96.0);

    // Test memory utilization
    let utilization = stats.memory_utilization();
    assert_eq!(utilization.total_allocated_bytes, 192);
    assert_eq!(utilization.largest_allocation, 128);
    assert_eq!(utilization.smallest_allocation, 64);
}

#[test]
fn test_forge_config_integration() {
    let config = ForgeConfig::optimized_for_small_allocations();
    let mut pool = config.create_pool().unwrap();

    assert!(pool.total_size() >= 64 * 1024);
    assert_eq!(pool.alignment_strategy(), AlignmentStrategy::CacheLine);

    // Should be able to allocate many small objects
    for i in 0..100 {
        let _ptr = pool.allocate(64, 8).unwrap();
        // With cache line strategy, allocation will be 64-byte aligned
        assert!(pool.current_offset() >= 64 * (i + 1));
    }
}

#[test]
fn test_architecture_specific_configs() {
    let architectures = [
        (TargetArchitecture::X86_64, AlignmentStrategy::CacheOptimized, 64),
        (TargetArchitecture::ARM64, AlignmentStrategy::CacheLine, 64),
        (TargetArchitecture::RISCV, AlignmentStrategy::Custom(32), 32),
        (TargetArchitecture::WebAssembly, AlignmentStrategy::Minimal, 8),
    ];

    for (arch, expected_strategy, expected_alignment) in architectures {
        let config = ForgeConfig::for_architecture(arch);
        let pool = config.create_pool().unwrap();

        assert_eq!(pool.alignment_strategy(), expected_strategy);
        assert_eq!(pool.base_alignment(), expected_alignment);

        // Test that we can allocate memory
        let mut test_pool = config.create_pool().unwrap();
        let _ptr = test_pool.allocate(100, 8).unwrap();
        assert!(!test_pool.is_empty());
    }
}

#[test]
fn test_current_architecture_detection() {
    let current_arch = TargetArchitecture::current();
    let config = ForgeConfig::for_architecture(current_arch);
    let pool = config.create_pool().unwrap();

    // Should create a valid pool for the current architecture
    assert!(pool.total_size() > 0);

    // Test cache line and page size properties
    assert!(current_arch.cache_line_size() > 0);
    assert!(current_arch.page_size() > 0);

    // Verify architecture-specific properties make sense
    match current_arch {
        TargetArchitecture::X86_64 | TargetArchitecture::ARM64 => {
            assert_eq!(current_arch.cache_line_size(), 64);
            assert_eq!(current_arch.page_size(), 4096);
        }
        TargetArchitecture::RISCV => {
            assert_eq!(current_arch.cache_line_size(), 32);
            assert_eq!(current_arch.page_size(), 4096);
        }
        TargetArchitecture::WebAssembly => {
            assert_eq!(current_arch.cache_line_size(), 8);
            assert_eq!(current_arch.page_size(), 64 * 1024);
        }
    }
}

#[test]
fn test_use_case_specific_configs() {
    // Test SIMD configuration
    let simd_config = ForgeConfig::optimized_for_simd(32);
    let simd_pool = simd_config.create_pool().unwrap();
    assert_eq!(simd_pool.alignment_strategy(), AlignmentStrategy::Custom(32));
    assert_eq!(simd_pool.base_alignment(), 32);

    // Test GPU configuration
    let gpu_config = ForgeConfig::optimized_for_gpu();
    let gpu_pool = gpu_config.create_pool().unwrap();
    assert_eq!(gpu_pool.alignment_strategy(), AlignmentStrategy::Custom(256));
    assert_eq!(gpu_pool.base_alignment(), 256);
    assert_eq!(gpu_config.default_pool_size, 32 * 1024 * 1024);

    // Test large data configuration
    let large_config = ForgeConfig::optimized_for_large_data();
    let large_pool = large_config.create_pool().unwrap();
    assert_eq!(large_pool.alignment_strategy(), AlignmentStrategy::Page);
    assert_eq!(large_pool.base_alignment(), 64);

    // Test minimal overhead configuration
    let minimal_config = ForgeConfig::minimal_overhead();
    let minimal_pool = minimal_config.create_pool().unwrap();
    assert_eq!(minimal_pool.alignment_strategy(), AlignmentStrategy::Minimal);
    assert!(!minimal_config.enable_stats); // Stats should be disabled
}

#[test]
fn test_concurrent_pool_usage() {
    // Test that pools can be created and used across threads (Send trait)
    let handles: Vec<_> = (0..4).map(|_| {
        thread::spawn(|| {
            let mut pool = MemoryPool::new(1024).unwrap();
            for _ in 0..10 {
                let _ptr = pool.allocate(64, 8).unwrap();
            }
            pool.get_stats().total_allocations
        })
    }).collect();

    for handle in handles {
        let allocation_count = handle.join().unwrap();
        assert_eq!(allocation_count, 10);
    }
}

#[test]
fn test_realistic_node_workflow_simulation() {
    // Simulate a typical node workflow: input -> process -> output
    let config = ForgeConfig::optimized_for_small_allocations();
    let mut pool = config.create_pool().unwrap();

    // Phase 1: Input data (simulate reading node inputs)
    let _input_data = pool.allocate(512, 8).unwrap(); // Input buffer
    let _metadata = pool.allocate(64, 8).unwrap();    // Metadata
    let phase1_checkpoint = pool.current_offset();

    // Phase 2: Processing (simulate computation)
    let _temp_buffer1 = pool.allocate(1024, 16).unwrap(); // Intermediate results
    let _temp_buffer2 = pool.allocate(256, 8).unwrap();   // Temporary calculations
    let _phase2_checkpoint = pool.current_offset();

    // Phase 3: Output preparation (simulate result formatting)
    let _output_buffer = pool.allocate(768, 8).unwrap(); // Final output

    // Verify we're using memory efficiently
    assert!(pool.current_offset() < pool.total_size());
    assert!(pool.efficiency() > 0.0); // Should be using some memory

    // Cleanup: Reset to phase1 (keep inputs, free temporaries)
    pool.reset_to(phase1_checkpoint).unwrap();

    // Verify we can reuse the memory
    let _reused_buffer = pool.allocate(512, 8).unwrap();
    assert!(pool.current_offset() >= phase1_checkpoint);

    let stats = pool.get_stats();
    assert!(stats.total_allocations >= 6);
    assert!(stats.reset_count >= 1);
}

#[test]
fn test_gpu_workflow_simulation() {
    // Simulate GPU rendering workflow
    let gpu_config = ForgeConfig::optimized_for_gpu();
    let mut gpu_pool = gpu_config.create_pool().unwrap();

    // Allocate GPU buffers with 256-byte alignment
    let vertex_buffer = gpu_pool.allocate(1024 * 1024, 256).unwrap(); // 1MB vertices
    let index_buffer = gpu_pool.allocate(512 * 1024, 256).unwrap();   // 512KB indices
    let texture_buffer = gpu_pool.allocate(2 * 1024 * 1024, 256).unwrap(); // 2MB texture

    // Verify all buffers are properly aligned for GPU DMA
    assert_eq!(vertex_buffer.as_ptr() as usize % 256, 0);
    assert_eq!(index_buffer.as_ptr() as usize % 256, 0);
    assert_eq!(texture_buffer.as_ptr() as usize % 256, 0);

    // Verify we can allocate and reset efficiently
    let checkpoint = gpu_pool.current_offset();
    let _temp_buffer = gpu_pool.allocate(1024 * 1024, 256).unwrap();
    gpu_pool.reset_to(checkpoint).unwrap(); // Free temporary buffer

    let stats = gpu_pool.get_stats();
    assert_eq!(stats.total_allocations, 4); // 3 persistent + 1 temporary
    assert_eq!(stats.reset_count, 1);
}

#[test]
fn test_simd_workflow_simulation() {
    // Simulate SIMD computation workflow
    let simd_config = ForgeConfig::optimized_for_simd(32); // AVX 256-bit
    let mut simd_pool = simd_config.create_pool().unwrap();

    // Allocate SIMD vectors
    let vector_a = simd_pool.allocate(1024, 32).unwrap(); // 1KB vector A
    let vector_b = simd_pool.allocate(1024, 32).unwrap(); // 1KB vector B
    let result_vector = simd_pool.allocate(1024, 32).unwrap(); // 1KB result

    // Verify AVX alignment
    assert_eq!(vector_a.as_ptr() as usize % 32, 0);
    assert_eq!(vector_b.as_ptr() as usize % 32, 0);
    assert_eq!(result_vector.as_ptr() as usize % 32, 0);

    // Simulate computation phases
    let computation_checkpoint = simd_pool.current_offset();

    // Temporary computation buffers
    let _temp1 = simd_pool.allocate(512, 32).unwrap();
    let _temp2 = simd_pool.allocate(512, 32).unwrap();

    // Clean up temporaries
    simd_pool.reset_to(computation_checkpoint).unwrap();

    assert_eq!(simd_pool.current_offset(), computation_checkpoint);
}

#[test]
fn test_memory_efficiency_comparison() {
    // Compare efficiency across different strategies
    let size = 10000; // 10KB
    let strategies = [
        AlignmentStrategy::Minimal,
        AlignmentStrategy::CacheLine,
        AlignmentStrategy::CacheOptimized,
        AlignmentStrategy::Custom(128),
    ];

    for strategy in strategies {
        let pool = MemoryPool::new_with_strategy(size, 8, strategy).unwrap();
        let efficiency = size as f32 / pool.total_size() as f32;

        // All strategies should be reasonably efficient
        match strategy {
            AlignmentStrategy::Minimal => assert!(efficiency > 0.95), // Should be very efficient
            _ => assert!(efficiency > 0.1), // Others may have more overhead
        }
    }
}

#[test]
fn test_error_handling_comprehensive() {
    // Test various error conditions

    // Invalid alignment in pool creation
    let result = MemoryPool::new_with_alignment(1024, 3);
    assert!(matches!(result, Err(PoolError::InvalidAlignment { alignment: 3 })));

    // Pool exhaustion
    let mut small_pool = MemoryPool::new_with_strategy(64, 8, AlignmentStrategy::Minimal).unwrap();
    let result = small_pool.allocate(100, 8);
    assert!(matches!(result, Err(PoolError::PoolExhausted { .. })));

    // Invalid reset offset
    let mut pool = MemoryPool::new(1024).unwrap();
    let result = pool.reset_to(2000);
    assert!(matches!(result, Err(PoolError::InvalidOffset { .. })));

    // Invalid allocation alignment
    let mut pool = MemoryPool::new(1024).unwrap();
    let result = pool.allocate(64, 6); // 6 is not power of 2
    assert!(matches!(result, Err(PoolError::InvalidAlignment { alignment: 6 })));
}

#[test]
fn test_config_pool_creation_with_size() {
    let config = ForgeConfig::optimized_for_gpu();

    // Create pools with different sizes using the same configuration
    let small_pool = config.create_pool_with_size(1024 * 1024).unwrap(); // 1MB
    let large_pool = config.create_pool_with_size(64 * 1024 * 1024).unwrap(); // 64MB

    // Both should have the same alignment strategy
    assert_eq!(small_pool.alignment_strategy(), config.alignment_strategy);
    assert_eq!(large_pool.alignment_strategy(), config.alignment_strategy);

    // But different sizes
    assert!(small_pool.total_size() >= 1024 * 1024);
    assert!(large_pool.total_size() >= 64 * 1024 * 1024);
    assert!(large_pool.total_size() > small_pool.total_size());
}

#[test]
fn test_performance_characteristics() {
    let mut pool = MemoryPool::new(1024 * 1024).unwrap(); // 1MB

    // Test rapid allocation performance
    let start_time = std::time::Instant::now();

    for i in 0..1000 {
        let _ptr = pool.allocate(64, 8).unwrap();

        // Every 100 allocations, reset to simulate stack-like usage
        if i % 100 == 99 {
            pool.reset();
        }
    }

    let elapsed = start_time.elapsed();

    // Should be very fast (this is a rough performance check)
    assert!(elapsed.as_millis() < 100, "Allocations took too long: {:?}", elapsed);

    let stats = pool.get_stats();
    assert_eq!(stats.total_allocations, 1000);
    assert_eq!(stats.reset_count, 10); // 1000 / 100 = 10 resets

    // Test allocation rate
    assert!(stats.allocation_rate() > 100.0); // Should be > 100 allocations/second
}

#[test]
fn test_integration_with_different_data_types() {
    // Simulate integration with different data precision types (like lights renderer)

    #[derive(Debug, Clone, Copy)]
    enum DataPrecision {
        F16,
        F32,
        F64,
    }

    // Choose optimal configuration based on data precision
    let precision = DataPrecision::F32;
    let (alignment, strategy) = match precision {
        DataPrecision::F16 => (16, AlignmentStrategy::CacheLine),     // Compact + cache-friendly
        DataPrecision::F32 => (32, AlignmentStrategy::CacheOptimized), // Balanced performance
        DataPrecision::F64 => (64, AlignmentStrategy::Custom(64)),    // High precision alignment
    };

    let config = ForgeConfig {
        default_pool_size: 4 * 1024 * 1024, // 4MB for rendering
        default_alignment: alignment,
        alignment_strategy: strategy,
        enable_stats: true,
    };

    let mut pool = config.create_pool().unwrap();

    // Allocate data appropriate for the precision
    let data_size = match precision {
        DataPrecision::F16 => 2, // 2 bytes per float
        DataPrecision::F32 => 4, // 4 bytes per float
        DataPrecision::F64 => 8, // 8 bytes per float
    };

    let num_elements = 1000;
    let buffer = pool.allocate(data_size * num_elements, alignment).unwrap();

    // Verify alignment matches precision requirements
    assert_eq!(buffer.as_ptr() as usize % alignment, 0);
    assert_eq!(pool.base_alignment(), alignment);
    assert_eq!(pool.alignment_strategy(), strategy);
}