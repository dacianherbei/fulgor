//! Performance benchmarks comparing memory pool allocation vs system malloc.

use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId, black_box};
use forge::memory::pool::MemoryPool;
use std::alloc::{GlobalAlloc, Layout, System};

fn bench_allocation_speed(c: &mut Criterion) {
    let mut group = c.benchmark_group("allocation_speed");

    // Test different allocation sizes
    let sizes = [8, 16, 32, 64, 128, 256, 512, 1024];

    for &size in &sizes {
        // Benchmark system malloc
        group.bench_with_input(BenchmarkId::new("system_malloc", size), &size, |b, &size| {
            b.iter(|| {
                let layout = Layout::from_size_align(size, 8).unwrap();
                unsafe {
                    let ptr = System.alloc(layout);
                    if !ptr.is_null() {
                        black_box(ptr);
                        System.dealloc(ptr, layout);
                    }
                }
            })
        });

        // Benchmark memory pool allocation
        group.bench_with_input(BenchmarkId::new("memory_pool", size), &size, |b, &size| {
            let mut pool = MemoryPool::new(1024 * 1024).unwrap();
            b.iter(|| {
                pool.reset(); // Reset for fair comparison
                let ptr = pool.allocate(size, 8).unwrap();
                black_box(ptr);
                // No explicit deallocation needed - pool reset handles it
            })
        });
    }

    group.finish();
}

fn bench_allocation_patterns(c: &mut Criterion) {
    let mut group = c.benchmark_group("allocation_patterns");

    // Benchmark sequential allocation pattern (common in node workflows)
    group.bench_function("sequential_malloc", |b| {
        b.iter(|| {
            let mut ptrs = Vec::new();

            // Allocate 100 objects of varying sizes
            for i in 0..100 {
                let size = (i % 8 + 1) * 64; // 64, 128, 192, ..., 512 bytes
                let layout = Layout::from_size_align(size, 8).unwrap();
                unsafe {
                    let ptr = System.alloc(layout);
                    if !ptr.is_null() {
                        ptrs.push((ptr, layout));
                    }
                }
            }

            // Deallocate all
            for (ptr, layout) in ptrs {
                unsafe {
                    System.dealloc(ptr, layout);
                }
            }
        })
    });

    group.bench_function("sequential_pool", |b| {
        b.iter(|| {
            let mut pool = MemoryPool::new(1024 * 1024).unwrap();

            // Allocate 100 objects of varying sizes
            for i in 0..100 {
                let size = (i % 8 + 1) * 64;
                let _ptr = pool.allocate(size, 8).unwrap();
                black_box(_ptr);
            }

            // Deallocate all with single reset
            pool.reset();
        })
    });

    group.finish();
}

fn bench_stack_like_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("stack_operations");

    // Benchmark malloc with stack-like allocation/deallocation
    group.bench_function("malloc_stack_simulation", |b| {
        b.iter(|| {
            let mut stack = Vec::new();

            // Simulate stack-like push operations
            for i in 0..50 {
                let size = 64 + (i % 4) * 32;
                let layout = Layout::from_size_align(size, 8).unwrap();
                unsafe {
                    let ptr = System.alloc(layout);
                    if !ptr.is_null() {
                        stack.push((ptr, layout));
                    }
                }
            }

            // Simulate stack-like pop operations (LIFO)
            while let Some((ptr, layout)) = stack.pop() {
                unsafe {
                    System.dealloc(ptr, layout);
                }
            }
        })
    });

    // Benchmark memory pool with stack-like operations
    group.bench_function("pool_stack_native", |b| {
        b.iter(|| {
            let mut pool = MemoryPool::new(1024 * 1024).unwrap();
            let mut checkpoints = Vec::new();

            // Push operations with checkpoints
            for i in 0..50 {
                let size = 64 + (i % 4) * 32;
                let _ptr = pool.allocate(size, 8).unwrap();
                checkpoints.push(pool.current_offset());
                black_box(_ptr);
            }

            // Pop operations using reset_to
            while let Some(checkpoint) = checkpoints.pop() {
                if checkpoint >= 64 {
                    pool.reset_to(checkpoint - 64).unwrap(); // "Pop" last allocation
                }
            }
        })
    });

    group.finish();
}

fn bench_memory_efficiency(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_efficiency");

    // Test fragmentation resistance
    group.bench_function("malloc_fragmentation", |b| {
        b.iter(|| {
            let mut ptrs = Vec::new();

            // Allocate objects of random sizes
            for i in 0..100 {
                let size = match i % 5 {
                    0 => 32,
                    1 => 128,
                    2 => 64,
                    3 => 256,
                    _ => 96,
                };
                let layout = Layout::from_size_align(size, 8).unwrap();
                unsafe {
                    let ptr = System.alloc(layout);
                    if !ptr.is_null() {
                        ptrs.push((ptr, layout));
                    }
                }
            }

            // Free every other allocation (create fragmentation)
            for i in (0..ptrs.len()).step_by(2) {
                unsafe {
                    System.dealloc(ptrs[i].0, ptrs[i].1);
                }
            }

            // Try to allocate large object (may fail due to fragmentation)
            let large_layout = Layout::from_size_align(8192, 8).unwrap();
            unsafe {
                let large_ptr = System.alloc(large_layout);
                if !large_ptr.is_null() {
                    System.dealloc(large_ptr, large_layout);
                }
            }

            // Clean up remaining allocations
            for i in (1..ptrs.len()).step_by(2) {
                unsafe {
                    System.dealloc(ptrs[i].0, ptrs[i].1);
                }
            }
        })
    });

    group.bench_function("pool_no_fragmentation", |b| {
        b.iter(|| {
            let mut pool = MemoryPool::new(1024 * 1024).unwrap();

            // Allocate objects of random sizes
            for i in 0..100 {
                let size = match i % 5 {
                    0 => 32,
                    1 => 128,
                    2 => 64,
                    3 => 256,
                    _ => 96,
                };
                let _ptr = pool.allocate(size, 8).unwrap();
                black_box(_ptr);
            }

            // Pool allocation is sequential - no fragmentation
            // Large allocation always succeeds if space available
            let checkpoint = pool.current_offset();
            if pool.available() >= 8192 {
                let _large_ptr = pool.allocate(8192, 8).unwrap();
                black_box(_large_ptr);
            }

            // Reset entire pool
            pool.reset();
        })
    });

    group.finish();
}

fn bench_cache_performance(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_performance");

    // Test cache-friendly sequential access
    group.bench_function("pool_sequential_access", |b| {
        let mut pool = MemoryPool::new(1024 * 1024).unwrap();

        // Pre-allocate memory regions
        let mut ptrs = Vec::new();
        for _ in 0..1000 {
            ptrs.push(pool.allocate(64, 8).unwrap());
        }

        b.iter(|| {
            // Sequential access pattern (cache-friendly)
            let mut sum = 0u64;
            for ptr in &ptrs {
                unsafe {
                    let val = ptr.as_ptr() as usize;
                    sum = sum.wrapping_add(val as u64);
                }
            }
            black_box(sum);
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_allocation_speed,
    bench_allocation_patterns,
    bench_stack_like_operations,
    bench_memory_efficiency,
    bench_cache_performance
);

criterion_main!(benches);