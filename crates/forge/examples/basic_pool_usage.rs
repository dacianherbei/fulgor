//! Basic memory pool usage example demonstrating stack-like allocation semantics.

use forge::memory::MemoryPool;

fn main() -> anyhow::Result<()> {
    println!("🔧 Basic Memory Pool Usage");
    println!("==========================");

    // Create 1MB pool
    let mut pool = MemoryPool::new(1024 * 1024)?;
    println!("✅ Created pool: {} bytes", pool.total_size());

    // Demonstrate basic allocation
    basic_allocation_demo(&mut pool)?;

    // Demonstrate stack-like semantics
    stack_semantics_demo(&mut pool)?;

    // Show performance statistics
    performance_stats_demo(&pool);

    Ok(())
}

fn basic_allocation_demo(pool: &mut MemoryPool) -> anyhow::Result<()> {
    println!("\n📦 Basic Allocation Demo");
    println!("------------------------");

    // Allocate various sizes with different alignments
    let ptr1 = pool.allocate(64, 8)?;   // 64 bytes, 8-byte aligned
    let ptr2 = pool.allocate(128, 16)?; // 128 bytes, 16-byte aligned
    let ptr3 = pool.allocate(256, 32)?; // 256 bytes, 32-byte aligned

    println!("Allocated 64 bytes at:  {:p}", ptr1.as_ptr());
    println!("Allocated 128 bytes at: {:p}", ptr2.as_ptr());
    println!("Allocated 256 bytes at: {:p}", ptr3.as_ptr());

    println!("Current usage: {} / {} bytes ({:.1}%)",
             pool.current_offset(),
             pool.total_size(),
             (pool.current_offset() as f32 / pool.total_size() as f32) * 100.0);

    // Verify alignment
    assert_eq!(ptr1.as_ptr() as usize % 8, 0, "8-byte alignment failed");
    assert_eq!(ptr2.as_ptr() as usize % 16, 0, "16-byte alignment failed");
    assert_eq!(ptr3.as_ptr() as usize % 32, 0, "32-byte alignment failed");
    println!("✅ All allocations properly aligned");

    Ok(())
}

fn stack_semantics_demo(pool: &mut MemoryPool) -> anyhow::Result<()> {
    println!("\n📚 Stack Semantics Demo");
    println!("-----------------------");

    // Reset pool first
    pool.reset();
    println!("Pool reset - available: {} bytes", pool.available());

    // Create a checkpoint
    let _ptr1 = pool.allocate(100, 8)?;
    let checkpoint = pool.current_offset();
    println!("Checkpoint at offset: {}", checkpoint);

    // Allocate more memory
    let _ptr2 = pool.allocate(200, 8)?;
    let _ptr3 = pool.allocate(300, 8)?;
    println!("After more allocations - offset: {}", pool.current_offset());

    // Reset to checkpoint (like unwinding stack)
    pool.reset_to(checkpoint)?;
    println!("Reset to checkpoint - offset: {}", pool.current_offset());

    // Allocate again (reusing "freed" memory)
    let _ptr4 = pool.allocate(150, 8)?;
    println!("New allocation - offset: {}", pool.current_offset());

    println!("✅ Stack-like semantics working correctly");

    Ok(())
}

fn performance_stats_demo(pool: &MemoryPool) {
    println!("\n📊 Performance Statistics");
    println!("-------------------------");

    let stats = pool.get_stats();

    println!("Total allocations: {}", stats.total_allocations);
    println!("Total bytes allocated: {}", stats.total_bytes_allocated);
    println!("Average allocation size: {:.1} bytes", stats.average_allocation_size);
    println!("Largest allocation: {} bytes", stats.largest_allocation);
    println!("Pool resets: {}", stats.reset_count);
    println!("High water mark: {} bytes", pool.high_water_mark());
    println!("Pool efficiency: {:.1}%", pool.efficiency() * 100.0);
    println!("Allocation rate: {:.0} allocs/sec", stats.allocation_rate());

    let utilization = stats.memory_utilization();
    println!("\n{}", utilization.format_summary());
}