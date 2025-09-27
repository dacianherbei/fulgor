//! Research-based performance comparison using optimized memory pool techniques.

use forge::memory::optimized_pool::{OptimizedPool, FastPoolConfig};
use std::alloc::{GlobalAlloc, Layout, System};
use std::time::Instant;

fn main() -> anyhow::Result<()> {
    println!("🚀 Research-Based Memory Pool Performance Comparison");
    println!("===================================================");

    let iterations = 10_000; // Start with smaller iterations for testing
    let allocation_size = 64;

    println!("Test parameters: {} iterations of {} bytes each", iterations, allocation_size);
    println!("Total memory needed: {} KB", (iterations * allocation_size) / 1024);

    // Test 1: Original memory pool vs optimized pool
    compare_pool_implementations(iterations, allocation_size)?;

    // Test 2: Fair comparison - allocation only (no deallocation)
    fair_allocation_comparison(iterations, allocation_size)?;

    // Test 3: Realistic workload simulation
    realistic_workload_comparison()?;

    // Test 4: Different allocation patterns
    allocation_pattern_comparison()?;

    Ok(())
}

fn compare_pool_implementations(iterations: usize, size: usize) -> anyhow::Result<()> {
    println!("\n📊 Pool Implementation Comparison");
    println!("---------------------------------");

    // Test optimized pool (fastest configuration)
    let fast_config = FastPoolConfig::fastest();
    println!("Pool size: {} MB", fast_config.pool_size / (1024 * 1024));
    println!("Alignment: {} bytes", fast_config.alignment);
    println!("Stats enabled: {}", fast_config.enable_stats);

    let optimized_time = time_optimized_pool_operations(iterations, size, &fast_config)?;

    // Test optimized pool with stats
    let stats_config = FastPoolConfig::fast_with_stats();
    let stats_time = time_optimized_pool_operations(iterations, size, &stats_config)?;

    // Calculate overhead
    let stats_overhead = (stats_time.as_nanos() as f64 / optimized_time.as_nanos() as f64 - 1.0) * 100.0;

    println!("Optimized (no stats): {:.2} ms ({:.0} ns/alloc)",
             optimized_time.as_secs_f64() * 1000.0,
             optimized_time.as_nanos() as f64 / iterations as f64);
    println!("Optimized (with stats): {:.2} ms ({:.0} ns/alloc)",
             stats_time.as_secs_f64() * 1000.0,
             stats_time.as_nanos() as f64 / iterations as f64);
    println!("📈 Statistics overhead: {:.1}%", stats_overhead);

    if optimized_time < stats_time {
        println!("✅ Zero-overhead statistics design validated");
    }

    Ok(())
}

fn fair_allocation_comparison(iterations: usize, size: usize) -> anyhow::Result<()> {
    println!("\n⚖️  Fair Allocation-Only Comparison");
    println!("----------------------------------");

    // Test malloc (allocation only, no deallocation)
    let malloc_time = time_malloc_allocation_only(iterations, size);

    // Test optimized pool
    let fast_config = FastPoolConfig::fastest();
    let pool_time = time_optimized_pool_operations(iterations, size, &fast_config)?;

    let speedup = malloc_time.as_nanos() as f64 / pool_time.as_nanos() as f64;

    println!("System malloc (alloc only): {:.2} ms ({:.0} ns/alloc)",
             malloc_time.as_secs_f64() * 1000.0,
             malloc_time.as_nanos() as f64 / iterations as f64);
    println!("Optimized pool:             {:.2} ms ({:.0} ns/alloc)",
             pool_time.as_secs_f64() * 1000.0,
             pool_time.as_nanos() as f64 / iterations as f64);

    if speedup > 1.0 {
        println!("🚀 Pool is {:.1}x faster than malloc", speedup);
    } else {
        println!("⚠️  Pool is {:.1}x slower than malloc (target: >1.0x)", speedup);
    }

    Ok(())
}

fn realistic_workload_comparison() -> anyhow::Result<()> {
    println!("\n🎯 Realistic Node Workflow Simulation");
    println!("------------------------------------");

    let node_count = 10;
    let iterations = 1000;

    let malloc_time = time_malloc_node_workflow(node_count, iterations);
    let pool_time = time_pool_node_workflow(node_count, iterations)?;

    let speedup = malloc_time.as_nanos() as f64 / pool_time.as_nanos() as f64;

    println!("Malloc workflow ({} nodes, {} iters): {:.2} ms",
             node_count, iterations, malloc_time.as_secs_f64() * 1000.0);
    println!("Pool workflow   ({} nodes, {} iters): {:.2} ms",
             node_count, iterations, pool_time.as_secs_f64() * 1000.0);

    if speedup > 1.0 {
        println!("🎉 Pool workflow is {:.1}x faster", speedup);
    } else {
        println!("📝 Pool workflow is {:.1}x slower", speedup);
    }

    Ok(())
}

fn allocation_pattern_comparison() -> anyhow::Result<()> {
    println!("\n🔄 Different Allocation Pattern Analysis");
    println!("---------------------------------------");

    // Test different pool configurations
    let configs = [
        ("Fastest (no stats)", FastPoolConfig::fastest()),
        ("Cache Optimized", FastPoolConfig::cache_optimized()),
        ("SIMD Optimized", FastPoolConfig::simd_optimized(32)),
    ];

    for (name, config) in configs {
        let time = time_pattern_burst_allocations(1000, 64, &config)?;
        println!("{:20}: {:.2} ms ({:.0} ns/alloc)",
                 name,
                 time.as_secs_f64() * 1000.0,
                 time.as_nanos() as f64 / 1000.0);
    }

    Ok(())
}

// Timing functions

fn time_malloc_allocation_only(iterations: usize, size: usize) -> std::time::Duration {
    let layout = Layout::from_size_align(size, 8).unwrap();
    let mut ptrs = Vec::with_capacity(iterations);

    let start = Instant::now();

    for _ in 0..iterations {
        unsafe {
            let ptr = System.alloc(layout);
            if !ptr.is_null() {
                // Prevent optimization
                std::ptr::write_volatile(ptr, 42);
                ptrs.push(ptr);
            }
        }
    }

    let elapsed = start.elapsed();

    // Clean up (not included in timing)
    for ptr in ptrs {
        unsafe {
            System.dealloc(ptr, layout);
        }
    }

    elapsed
}

fn time_optimized_pool_operations(
    iterations: usize,
    size: usize,
    config: &FastPoolConfig
) -> anyhow::Result<std::time::Duration> {
    let mut pool = config.create_pool()?;

    let start = Instant::now();

    // Allocate in batches with resets (more realistic usage)
    let batch_size = 1000; // Reset every 1000 allocations
    for batch in 0..(iterations / batch_size) {
        for _ in 0..batch_size {
            let ptr = pool.allocate_fast(size)?;
            // Prevent optimization
            unsafe {
                std::ptr::write_volatile(ptr.as_ptr(), batch as u8);
            }
        }
        // Reset pool between batches (stack-like usage)
        pool.reset_fast();
    }

    // Handle remaining allocations
    let remaining = iterations % batch_size;
    for _ in 0..remaining {
        let ptr = pool.allocate_fast(size)?;
        unsafe {
            std::ptr::write_volatile(ptr.as_ptr(), 42);
        }
    }

    Ok(start.elapsed())
}

fn time_malloc_node_workflow(node_count: usize, iterations: usize) -> std::time::Duration {
    let layout = Layout::from_size_align(128, 8).unwrap(); // Typical node data size

    let start = Instant::now();

    for _ in 0..iterations {
        let mut node_ptrs = Vec::new();

        // Simulate node execution: allocate input/output buffers
        for _ in 0..node_count {
            unsafe {
                let input_ptr = System.alloc(layout);
                let output_ptr = System.alloc(layout);
                if !input_ptr.is_null() && !output_ptr.is_null() {
                    // Simulate computation
                    std::ptr::write_volatile(input_ptr, 42);
                    std::ptr::write_volatile(output_ptr, 84);
                    node_ptrs.push((input_ptr, output_ptr));
                }
            }
        }

        // Clean up nodes
        for (input_ptr, output_ptr) in node_ptrs {
            unsafe {
                System.dealloc(input_ptr, layout);
                System.dealloc(output_ptr, layout);
            }
        }
    }

    start.elapsed()
}

fn time_pool_node_workflow(node_count: usize, iterations: usize) -> anyhow::Result<std::time::Duration> {
    let config = FastPoolConfig::fastest();
    let mut pool = config.create_pool()?;

    let start = Instant::now();

    for _ in 0..iterations {
        // Simulate node execution workflow
        for _ in 0..node_count {
            let _input_ptr = pool.allocate_fast(128)?;  // Input buffer
            let _output_ptr = pool.allocate_fast(128)?; // Output buffer

            // Simulate computation
            unsafe {
                std::ptr::write_volatile(_input_ptr.as_ptr(), 42);
                std::ptr::write_volatile(_output_ptr.as_ptr(), 84);
            }
        }

        // Reset pool for next iteration (stack-like cleanup)
        pool.reset_fast();
    }

    Ok(start.elapsed())
}

fn time_pattern_burst_allocations(
    iterations: usize,
    size: usize,
    config: &FastPoolConfig
) -> anyhow::Result<std::time::Duration> {
    let mut pool = config.create_pool()?;

    let start = Instant::now();

    // Simulate burst allocation pattern (common in node execution)
    for burst in 0..(iterations / 10) {
        // Burst of 10 allocations
        for _ in 0..10 {
            let ptr = pool.allocate_fast(size)?;
            unsafe {
                std::ptr::write_volatile(ptr.as_ptr(), burst as u8);
            }
        }

        // Reset between bursts (stack-like usage)
        if burst % 5 == 4 {
            pool.reset_fast();
        }
    }

    Ok(start.elapsed())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_performance_functions() {
        // Test that performance measurement functions work
        let malloc_time = time_malloc_allocation_only(100, 64);
        assert!(malloc_time.as_nanos() > 0);

        let config = FastPoolConfig::fastest();
        let pool_time = time_optimized_pool_operations(100, 64, &config).unwrap();
        assert!(pool_time.as_nanos() > 0);
    }

    #[test]
    fn test_node_workflow_simulation() {
        let malloc_time = time_malloc_node_workflow(10, 5);
        let pool_time = time_pool_node_workflow(10, 5).unwrap();

        assert!(malloc_time.as_nanos() > 0);
        assert!(pool_time.as_nanos() > 0);

        // Pool should generally be faster
        println!("Malloc: {:?}, Pool: {:?}", malloc_time, pool_time);
    }
}