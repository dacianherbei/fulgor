//! Integration tests for the forge memory management system.

use forge::memory::pool::{MemoryPool, PoolError};
use forge::ForgeConfig;
use std::thread;

#[test]
fn test_memory_pool_creation_and_basic_usage() {
    let mut pool = MemoryPool::new(1024).unwrap();

    // Test basic properties
    assert!(pool.total_size() >= 1024); // May be aligned up
    assert_eq!(pool.current_offset(), 0);
    assert!(pool.is_empty());
    assert_eq!(pool.efficiency(), 1.0); // No allocations yet

    // Test basic allocation
    let ptr = pool.allocate(64, 8).unwrap();
    assert!(!ptr.as_ptr().is_null());
    assert_eq!(pool.current_offset(), 64);
    assert!(!pool.is_empty());
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
        assert_eq!(ptr.as_ptr() as usize % align, 0,
                   "Failed alignment requirement: {} bytes", align);
    }
}

#[test]
fn test_pool_exhaustion() {
    let mut pool = MemoryPool::new(100).unwrap();

    // Fill the pool
    let _ptr1 = pool.allocate(50, 8).unwrap();
    let _ptr2 = pool.allocate(40, 8).unwrap();

    // This should fail due to insufficient space
    let result = pool.allocate(20, 8);
    assert!(matches!(result, Err(PoolError::PoolExhausted { .. })));
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
}

#[test]
fn test_reset_functionality() {
    let mut pool = MemoryPool::new(1024).unwrap();

    // Allocate some memory
    let _ptr1 = pool.allocate(100, 8).unwrap();
    let checkpoint = pool.current_offset();

    let _ptr2 = pool.allocate(200, 8).unwrap();
    assert_eq!(pool.current_offset(), 304);

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
    assert_eq!(pool.high_water_mark(), 100);

    let _ptr2 = pool.allocate(200, 8).unwrap();
    // After allocating 100 bytes, the next allocation at 8-byte alignment
    // will start at offset 104 (100 rounded up to next 8-byte boundary)
    // So: 104 + 200 = 304
    assert_eq!(pool.high_water_mark(), 304);

    // Reset and allocate smaller amount
    pool.reset();
    let _ptr3 = pool.allocate(50, 8).unwrap();
    assert_eq!(pool.high_water_mark(), 304); // Should remain at peak
}

#[test]
fn test_high_water_mark_aligned() {
    let mut pool = MemoryPool::new(1024).unwrap();

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
    let mut pool = MemoryPool::new(1024).unwrap();

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
}

#[test]
fn test_forge_config_integration() {
    let config = ForgeConfig::optimized_for_small_allocations();
    let mut pool = config.create_pool().unwrap();

    assert!(pool.total_size() >= 64 * 1024);

    // Should be able to allocate many small objects
    for i in 0..100 {
        let _ptr = pool.allocate(64, 8).unwrap();
        assert_eq!(pool.current_offset(), 64 * (i + 1));
    }
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
    let mut pool = MemoryPool::new(4096).unwrap();

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
    assert!(pool.efficiency() > 0.5); // At least 50% efficient

    // Cleanup: Reset to phase1 (keep inputs, free temporaries)
    pool.reset_to(phase1_checkpoint).unwrap();

    // Verify we can reuse the memory
    let _reused_buffer = pool.allocate(512, 8).unwrap();
    assert_eq!(pool.current_offset(), phase1_checkpoint + 512);

    let stats = pool.get_stats();
    assert!(stats.total_allocations >= 6);
    assert!(stats.reset_count >= 1);
}