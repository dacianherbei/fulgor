//! Simple performance comparison between malloc and memory pool allocation.

use forge::memory::pool::MemoryPool;
use std::alloc::{GlobalAlloc, Layout, System};
use std::time::Instant;

fn main() -> anyhow::Result<()> {
    println!("🚀 Memory Pool Performance Comparison");
    println!("=====================================");

    let iterations = 10_000;
    let allocation_size = 64;

    // Test system malloc performance
    let malloc_time = time_malloc_operations(iterations, allocation_size);

    // Test memory pool performance
    let pool_time = time_pool_operations(iterations, allocation_size)?;

    // Calculate and display results
    let speedup = malloc_time.as_nanos() as f64 / pool_time.as_nanos() as f64;

    println!("\n📊 Results ({} iterations, {} bytes each):", iterations, allocation_size);
    println!("System malloc: {:.2} ms ({:.0} ns/alloc)",
             malloc_time.as_secs_f64() * 1000.0,
             malloc_time.as_nanos() as f64 / iterations as f64);
    println!("Memory pool:   {:.2} ms ({:.0} ns/alloc)",
             pool_time.as_secs_f64() * 1000.0,
             pool_time.as_nanos() as f64 / iterations as f64);
    println!("Speedup:       {:.1}x faster", speedup);

    if speedup > 2.0 {
        println!("✅ Memory pool provides significant performance improvement!");
    } else if speedup > 1.0 {
        println!("✅ Memory pool is faster than malloc");
    } else {
        println!("⚠️  Memory pool performance needs optimization");
    }

    Ok(())
}

fn time_malloc_operations(iterations: usize, size: usize) -> std::time::Duration {
    let layout = Layout::from_size_align(size, 8).unwrap();

    let start = Instant::now();

    for _ in 0..iterations {
        unsafe {
            let ptr = System.alloc(layout);
            if !ptr.is_null() {
                // Prevent optimization
                std::ptr::write_volatile(ptr, 42);
                System.dealloc(ptr, layout);
            }
        }
    }

    start.elapsed()
}

fn time_pool_operations(iterations: usize, size: usize) -> anyhow::Result<std::time::Duration> {
    let mut pool = MemoryPool::new(size * iterations * 2)?; // Extra space for alignment

    let start = Instant::now();

    for _ in 0..iterations {
        let ptr = pool.allocate(size, 8)?;
        // Prevent optimization
        unsafe {
            std::ptr::write_volatile(ptr.as_ptr(), 42);
        }
        // Note: No explicit deallocation - this is the key advantage
    }

    Ok(start.elapsed())
}