//! Simple test to verify the optimized pool works correctly

use forge::memory::optimized_pool::{OptimizedPool, FastPoolConfig};

fn main() -> anyhow::Result<()> {
    println!("🧪 Simple Optimized Pool Test");
    println!("=============================");

    // Test 1: Basic allocation
    println!("\n1. Testing basic allocation...");
    let config = FastPoolConfig::fastest();
    let mut pool = config.create_pool()?;

    println!("   Pool size: {} KB", pool.total_size() / 1024);
    println!("   Initial usage: {} bytes", pool.current_usage());

    // Allocate some memory
    let ptr1 = pool.allocate_fast(64)?;
    println!("   After 64-byte allocation: {} bytes used", pool.current_usage());

    let ptr2 = pool.allocate_fast(128)?;
    println!("   After 128-byte allocation: {} bytes used", pool.current_usage());

    // Verify pointers are different
    assert_ne!(ptr1.as_ptr(), ptr2.as_ptr());
    println!("   ✅ Allocations successful and distinct");

    // Test 2: Pool reset
    println!("\n2. Testing pool reset...");
    let usage_before_reset = pool.current_usage();
    pool.reset_fast();
    println!("   Usage before reset: {} bytes", usage_before_reset);
    println!("   Usage after reset: {} bytes", pool.current_usage());
    assert_eq!(pool.current_usage(), 0);
    println!("   ✅ Pool reset successful");

    // Test 3: Statistics
    println!("\n3. Testing statistics...");
    let stats_config = FastPoolConfig::fast_with_stats();
    let mut stats_pool = stats_config.create_pool()?;

    assert_eq!(stats_pool.allocation_count(), 0);
    let _ptr = stats_pool.allocate_fast(32)?;
    let _ptr = stats_pool.allocate_fast(32)?;
    println!("   Allocation count: {}", stats_pool.allocation_count());
    assert_eq!(stats_pool.allocation_count(), 2);
    println!("   ✅ Statistics tracking working");

    // Test 4: Batch allocations
    println!("\n4. Testing batch allocations...");
    let mut batch_pool = FastPoolConfig::fastest().create_pool()?;
    let mut allocation_count = 0;

    for batch in 0..10 {
        for _ in 0..100 {
            let _ptr = batch_pool.allocate_fast(64)?;
            allocation_count += 1;
        }
        batch_pool.reset_fast();
        if batch == 0 {
            println!("   Completed first batch of 100 allocations");
        }
    }

    println!("   Total allocations: {}", allocation_count);
    println!("   ✅ Batch allocations successful");

    println!("\n🎉 All tests passed! Optimized pool is working correctly.");

    Ok(())
}