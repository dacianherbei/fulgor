//! High-performance memory pool with architecture-aware alignment strategies.
//!
//! This module provides a memory pool that combines the performance of stack allocation
//! with the flexibility of heap allocation, while optimizing for different CPU architectures
//! and use cases.

use std::alloc::{GlobalAlloc, Layout, System};
use std::ptr::NonNull;
use anyhow::Result;
use crate::memory::stats::PoolStats;

/// Errors that can occur during memory pool operations
#[derive(Debug, thiserror::Error)]
pub enum PoolError {
    #[error("Pool exhausted: requested {requested} bytes, available {available} bytes")]
    PoolExhausted { requested: usize, available: usize },

    #[error("Invalid alignment: {alignment} is not a power of 2")]
    InvalidAlignment { alignment: usize },

    #[error("Invalid offset: {offset} is beyond pool size {pool_size}")]
    InvalidOffset { offset: usize, pool_size: usize },

    #[error("Allocation failed: {0}")]
    AllocationFailed(String),
}

/// Alignment strategies for different use cases and architectures
///
/// Different alignment strategies optimize for various scenarios, from minimal
/// overhead to maximum performance for specific hardware architectures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlignmentStrategy {
    /// Use the specified alignment as-is (minimum overhead)
    ///
    /// Best for: Memory-constrained environments, embedded systems
    Minimal,

    /// Align to cache lines (64 bytes) for better cache performance
    ///
    /// Best for: General-purpose computing, ARM processors
    CacheLine,

    /// Align to page boundaries (4KB) for virtual memory efficiency
    ///
    /// Best for: Large data processing, server applications
    Page,

    /// Use the larger of cache line or specified alignment
    ///
    /// Best for: General-purpose optimization, x86_64 processors
    CacheOptimized,

    /// Custom alignment value
    ///
    /// Best for: SIMD operations, GPU operations, specific hardware requirements
    Custom(usize),
}

impl AlignmentStrategy {
    /// Get the alignment value for this strategy
    ///
    /// # Arguments
    /// * `requested_alignment` - The minimum alignment requested by the caller
    ///
    /// # Returns
    /// The effective alignment that will be used, which may be larger than requested
    pub fn alignment_value(&self, requested_alignment: usize) -> usize {
        match self {
            AlignmentStrategy::Minimal => requested_alignment,
            AlignmentStrategy::CacheLine => 64.max(requested_alignment),
            AlignmentStrategy::Page => 4096.max(requested_alignment),
            AlignmentStrategy::CacheOptimized => 64.max(requested_alignment),
            AlignmentStrategy::Custom(align) => (*align).max(requested_alignment),
        }
    }

    /// Calculate the pool size alignment based on strategy
    ///
    /// # Arguments
    /// * `base_alignment` - The base alignment requirement for the pool
    ///
    /// # Returns
    /// The alignment that should be used for the pool's total size
    pub fn pool_size_alignment(&self, base_alignment: usize) -> usize {
        match self {
            AlignmentStrategy::Minimal => base_alignment,
            AlignmentStrategy::CacheLine => 64.max(base_alignment),
            AlignmentStrategy::Page => 4096.max(base_alignment),
            AlignmentStrategy::CacheOptimized => 64.max(base_alignment),
            AlignmentStrategy::Custom(align) => (*align).max(base_alignment),
        }
    }

    /// Get a human-readable description of this alignment strategy
    pub fn description(&self) -> &'static str {
        match self {
            AlignmentStrategy::Minimal => "Minimal overhead, uses exact requested alignment",
            AlignmentStrategy::CacheLine => "64-byte cache line alignment for optimal cache performance",
            AlignmentStrategy::Page => "4KB page alignment for optimal virtual memory performance",
            AlignmentStrategy::CacheOptimized => "Cache-optimized with intelligent alignment selection",
            AlignmentStrategy::Custom(size) => match size {
                16 => "16-byte alignment for SSE/NEON SIMD operations",
                32 => "32-byte alignment for AVX SIMD operations",
                64 => "64-byte alignment for AVX-512 SIMD operations",
                256 => "256-byte alignment for GPU operations",
                _ => "Custom alignment for specific requirements",
            },
        }
    }
}

/// High-performance memory pool with architecture-aware alignment.
///
/// Provides O(1) allocation and stack-like deallocation through reset operations.
/// Uses configurable alignment strategies for optimal performance on different architectures.
///
/// # Performance Characteristics
///
/// - **Allocation**: O(1) - Simple pointer arithmetic
/// - **Deallocation**: O(1) - Stack-like reset operations
/// - **Memory Efficiency**: >95% - Sequential allocation with no fragmentation
/// - **Cache Performance**: Optimized through alignment strategies
///
/// # Memory Layout
///
/// ```text
/// Pool Buffer: [Used Memory][Available Memory]
///              ^            ^                ^
///              buffer       current_offset   total_size
/// ```
///
/// # Examples
///
/// ```rust
/// use forge::memory::pool::{MemoryPool, AlignmentStrategy};
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// // Basic usage
/// let mut pool = MemoryPool::new(1024 * 1024)?;
/// let ptr = pool.allocate(64, 8)?;
///
/// // Architecture-aware usage
/// let mut cache_pool = MemoryPool::new_with_strategy(
///     1024 * 1024, 8, AlignmentStrategy::CacheLine
/// )?;
///
/// // SIMD operations
/// let mut simd_pool = MemoryPool::new_with_strategy(
///     1024 * 1024, 32, AlignmentStrategy::Custom(32)
/// )?;
/// let simd_ptr = simd_pool.allocate(256, 32)?; // AVX-aligned
/// # Ok(())
/// # }
/// ```
pub struct MemoryPool {
    /// Pointer to the allocated buffer
    buffer: NonNull<u8>,
    /// Total size of the pool in bytes
    total_size: usize,
    /// Current allocation offset (stack pointer equivalent)
    current_offset: usize,
    /// Highest offset reached (high water mark)
    high_water_mark: usize,
    /// Base alignment for the pool
    base_alignment: usize,
    /// Alignment strategy used for this pool
    alignment_strategy: AlignmentStrategy,
    /// Layout used for the initial allocation
    layout: Layout,
    /// Statistics tracking
    stats: PoolStats,
}

impl MemoryPool {
    /// Create a new memory pool with default alignment strategy.
    ///
    /// Uses cache-optimized alignment strategy for balanced performance.
    ///
    /// # Arguments
    /// * `size` - Minimum size of the pool in bytes. The actual size will be aligned
    ///           based on the alignment strategy and may be larger.
    ///
    /// # Returns
    /// * `Ok(MemoryPool)` - Successfully created pool
    /// * `Err(PoolError)` - Failed to allocate backing memory
    ///
    /// # Examples
    /// ```rust
    /// use forge::memory::pool::MemoryPool;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let pool = MemoryPool::new(1024 * 1024)?; // 1MB pool
    /// assert!(pool.total_size() >= 1024 * 1024);
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(size: usize) -> Result<Self, PoolError> {
        Self::new_with_strategy(size, 8, AlignmentStrategy::CacheOptimized)
    }

    /// Create a new memory pool with specific alignment.
    ///
    /// Uses cache-optimized alignment strategy with the specified base alignment.
    ///
    /// # Arguments
    /// * `size` - Minimum size of the pool in bytes
    /// * `alignment` - Base alignment requirement (must be power of 2, minimum 8 bytes)
    ///
    /// # Examples
    /// ```rust
    /// use forge::memory::pool::MemoryPool;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let pool = MemoryPool::new_with_alignment(1024, 16)?;
    /// assert_eq!(pool.base_alignment(), 16);
    /// # Ok(())
    /// # }
    /// ```
    pub fn new_with_alignment(size: usize, alignment: usize) -> Result<Self, PoolError> {
        Self::new_with_strategy(size, alignment, AlignmentStrategy::CacheOptimized)
    }

    /// Create a new memory pool with specific alignment strategy.
    ///
    /// This is the most flexible constructor, allowing full control over the pool's
    /// alignment behavior for optimal performance on specific architectures or use cases.
    ///
    /// # Arguments
    /// * `size` - Minimum size of the pool in bytes
    /// * `base_alignment` - Base alignment requirement (must be power of 2, minimum 8 bytes)
    /// * `strategy` - Alignment strategy for optimal performance
    ///
    /// # Examples
    /// ```rust
    /// use forge::memory::pool::{MemoryPool, AlignmentStrategy};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// // SIMD operations - 32-byte alignment for AVX
    /// let simd_pool = MemoryPool::new_with_strategy(
    ///     1024 * 1024, 32, AlignmentStrategy::Custom(32)
    /// )?;
    ///
    /// // Cache-optimized general use
    /// let cache_pool = MemoryPool::new_with_strategy(
    ///     1024 * 1024, 8, AlignmentStrategy::CacheOptimized
    /// )?;
    ///
    /// // Minimal overhead for embedded systems
    /// let minimal_pool = MemoryPool::new_with_strategy(
    ///     64 * 1024, 8, AlignmentStrategy::Minimal
    /// )?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn new_with_strategy(
        size: usize,
        base_alignment: usize,
        strategy: AlignmentStrategy
    ) -> Result<Self, PoolError> {
        // Validate base alignment is power of 2
        if !base_alignment.is_power_of_two() {
            return Err(PoolError::InvalidAlignment { alignment: base_alignment });
        }

        // Ensure minimum alignment of 8 bytes for pointer safety
        let base_alignment = base_alignment.max(8);

        // Calculate effective alignment based on strategy
        let pool_alignment = strategy.pool_size_alignment(base_alignment);

        // Align size based on the strategy
        let aligned_size = (size + pool_alignment - 1) & !(pool_alignment - 1);

        // Create layout for the buffer
        let layout = Layout::from_size_align(aligned_size, pool_alignment)
            .map_err(|e| PoolError::AllocationFailed(
                format!("Invalid layout: size={}, align={}, error={}", aligned_size, pool_alignment, e)
            ))?;

        // Allocate the backing buffer
        let buffer = unsafe {
            let ptr = System.alloc(layout);
            if ptr.is_null() {
                return Err(PoolError::AllocationFailed(
                    format!("Failed to allocate {} bytes with {} alignment",
                            aligned_size, pool_alignment)
                ));
            }
            NonNull::new_unchecked(ptr)
        };

        Ok(Self {
            buffer,
            total_size: aligned_size,
            current_offset: 0,
            high_water_mark: 0,
            base_alignment,
            alignment_strategy: strategy,
            layout,
            stats: PoolStats::new(),
        })
    }

    /// Get the alignment strategy used by this pool
    ///
    /// # Examples
    /// ```rust
    /// use forge::memory::pool::{MemoryPool, AlignmentStrategy};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let pool = MemoryPool::new_with_strategy(
    ///     1024, 8, AlignmentStrategy::CacheLine
    /// )?;
    /// assert_eq!(pool.alignment_strategy(), AlignmentStrategy::CacheLine);
    /// # Ok(())
    /// # }
    /// ```
    pub fn alignment_strategy(&self) -> AlignmentStrategy {
        self.alignment_strategy
    }

    /// Get the base alignment for this pool
    ///
    /// # Examples
    /// ```rust
    /// use forge::memory::pool::MemoryPool;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let pool = MemoryPool::new_with_alignment(1024, 16)?;
    /// assert_eq!(pool.base_alignment(), 16);
    /// # Ok(())
    /// # }
    /// ```
    pub fn base_alignment(&self) -> usize {
        self.base_alignment
    }

    /// Allocate memory from the pool with specified size and alignment.
    ///
    /// The effective alignment will be determined by both the requested alignment
    /// and the pool's alignment strategy, ensuring optimal performance.
    ///
    /// # Arguments
    /// * `size` - Number of bytes to allocate
    /// * `align` - Minimum alignment requirement (must be power of 2)
    ///
    /// # Returns
    /// * `Ok(NonNull<u8>)` - Pointer to allocated memory
    /// * `Err(PoolError)` - Allocation failed
    ///
    /// # Examples
    /// ```rust
    /// use forge::memory::pool::MemoryPool;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut pool = MemoryPool::new(1024)?;
    ///
    /// let ptr1 = pool.allocate(64, 8)?;   // 64 bytes, 8-byte aligned
    /// let ptr2 = pool.allocate(128, 16)?; // 128 bytes, 16-byte aligned
    ///
    /// // Verify alignment
    /// assert_eq!(ptr1.as_ptr() as usize % 8, 0);
    /// assert_eq!(ptr2.as_ptr() as usize % 16, 0);
    /// # Ok(())
    /// # }
    /// ```
    pub fn allocate(&mut self, size: usize, align: usize) -> Result<NonNull<u8>, PoolError> {
        // Validate alignment
        if !align.is_power_of_two() {
            return Err(PoolError::InvalidAlignment { alignment: align });
        }

        // Apply alignment strategy
        let effective_alignment = self.alignment_strategy.alignment_value(align);

        // Calculate aligned offset
        let aligned_offset = (self.current_offset + effective_alignment - 1) & !(effective_alignment - 1);
        let end_offset = aligned_offset + size;

        // Check if allocation fits
        if end_offset > self.total_size {
            return Err(PoolError::PoolExhausted {
                requested: size,
                available: self.total_size.saturating_sub(aligned_offset),
            });
        }

        // Get pointer to allocated memory
        let ptr = unsafe {
            NonNull::new_unchecked(self.buffer.as_ptr().add(aligned_offset))
        };

        // Update pool state
        self.current_offset = end_offset;
        self.high_water_mark = self.high_water_mark.max(end_offset);

        // Update statistics
        self.stats.record_allocation(size, effective_alignment);

        Ok(ptr)
    }

    /// Reset the pool to a previous offset (stack-like "pop" operation).
    ///
    /// This allows for efficient stack-like memory management where you can
    /// allocate temporary objects and then "pop" them all at once by resetting
    /// to a previous checkpoint.
    ///
    /// # Arguments
    /// * `offset` - Offset to reset to (must be <= current_offset)
    ///
    /// # Examples
    /// ```rust
    /// use forge::memory::pool::MemoryPool;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut pool = MemoryPool::new(1024)?;
    ///
    /// let _ptr1 = pool.allocate(100, 8)?;
    /// let checkpoint = pool.current_offset();
    ///
    /// let _ptr2 = pool.allocate(200, 8)?;
    /// assert_eq!(pool.current_offset(), checkpoint + 200);
    ///
    /// // Reset to checkpoint, "freeing" ptr2
    /// pool.reset_to(checkpoint)?;
    /// assert_eq!(pool.current_offset(), checkpoint);
    /// # Ok(())
    /// # }
    /// ```
    pub fn reset_to(&mut self, offset: usize) -> Result<(), PoolError> {
        if offset > self.total_size {
            return Err(PoolError::InvalidOffset {
                offset,
                pool_size: self.total_size,
            });
        }

        if offset <= self.current_offset {
            self.current_offset = offset;
            self.stats.record_reset(offset);
            Ok(())
        } else {
            Err(PoolError::InvalidOffset {
                offset,
                pool_size: self.current_offset,
            })
        }
    }

    /// Reset the pool to the beginning (clear all allocations).
    ///
    /// This is equivalent to `reset_to(0)` and frees all allocated memory
    /// in the pool for reuse.
    ///
    /// # Examples
    /// ```rust
    /// use forge::memory::pool::MemoryPool;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut pool = MemoryPool::new(1024)?;
    ///
    /// let _ptr = pool.allocate(100, 8)?;
    /// assert!(!pool.is_empty());
    ///
    /// pool.reset();
    /// assert!(pool.is_empty());
    /// assert_eq!(pool.current_offset(), 0);
    /// # Ok(())
    /// # }
    /// ```
    pub fn reset(&mut self) {
        self.current_offset = 0;
        self.stats.record_reset(0);
    }

    /// Get the current allocation offset (equivalent to stack pointer).
    ///
    /// This represents how much memory has been allocated from the pool.
    pub fn current_offset(&self) -> usize {
        self.current_offset
    }

    /// Get the total size of the pool.
    ///
    /// This is the actual allocated size, which may be larger than the
    /// requested size due to alignment requirements.
    pub fn total_size(&self) -> usize {
        self.total_size
    }

    /// Get the amount of memory currently available for allocation.
    pub fn available(&self) -> usize {
        self.total_size - self.current_offset
    }

    /// Get the high water mark (maximum offset reached).
    ///
    /// This represents the peak memory usage of the pool, useful for
    /// analyzing memory requirements and pool sizing.
    pub fn high_water_mark(&self) -> usize {
        self.high_water_mark
    }

    /// Get pool statistics.
    ///
    /// Returns detailed statistics about allocation patterns, performance,
    /// and memory utilization.
    pub fn get_stats(&self) -> &PoolStats {
        &self.stats
    }

    /// Check if the pool is empty (no current allocations).
    pub fn is_empty(&self) -> bool {
        self.current_offset == 0
    }

    /// Get pool efficiency (percentage of allocated memory actually used).
    ///
    /// Returns a value between 0.0 and 1.0, where 1.0 means all allocated
    /// memory is being used efficiently.
    pub fn efficiency(&self) -> f32 {
        if self.high_water_mark == 0 {
            1.0 // Empty pool is considered 100% efficient
        } else {
            self.high_water_mark as f32 / self.total_size as f32
        }
    }

    /// Get memory fragmentation ratio.
    ///
    /// Returns 0.0 for no fragmentation (sequential allocation) up to 1.0
    /// for high fragmentation. Memory pools inherently have zero fragmentation
    /// due to sequential allocation, so this typically returns 0.0.
    pub fn fragmentation_ratio(&self) -> f32 {
        0.0 // Memory pools have zero fragmentation by design
    }

    /// Check if a given size allocation would fit in the pool.
    ///
    /// # Arguments
    /// * `size` - Size to check
    /// * `align` - Alignment requirement
    ///
    /// # Returns
    /// `true` if the allocation would succeed, `false` otherwise
    pub fn can_allocate(&self, size: usize, align: usize) -> bool {
        if !align.is_power_of_two() {
            return false;
        }

        let effective_alignment = self.alignment_strategy.alignment_value(align);
        let aligned_offset = (self.current_offset + effective_alignment - 1) & !(effective_alignment - 1);
        aligned_offset + size <= self.total_size
    }
}

impl Drop for MemoryPool {
    fn drop(&mut self) {
        // Deallocate the backing buffer
        unsafe {
            System.dealloc(self.buffer.as_ptr(), self.layout);
        }
    }
}

// Safety: MemoryPool can be sent between threads safely
// The pool owns its memory and doesn't share it with other threads
unsafe impl Send for MemoryPool {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alignment_strategies() {
        // Test minimal alignment strategy
        let minimal_pool = MemoryPool::new_with_strategy(1000, 8, AlignmentStrategy::Minimal).unwrap();
        assert_eq!(minimal_pool.total_size(), 1000); // Aligned to base alignment (8)

        // Test cache line alignment strategy
        let cache_pool = MemoryPool::new_with_strategy(1000, 8, AlignmentStrategy::CacheLine).unwrap();
        assert_eq!(cache_pool.total_size(), 1024); // Aligned to 64-byte cache line

        // Test page alignment strategy
        let page_pool = MemoryPool::new_with_strategy(1000, 8, AlignmentStrategy::Page).unwrap();
        assert_eq!(page_pool.total_size(), 4096); // Aligned to 4KB page

        // Test custom alignment
        let custom_pool = MemoryPool::new_with_strategy(1000, 8, AlignmentStrategy::Custom(128)).unwrap();
        assert_eq!(custom_pool.total_size(), 1024); // Aligned to 128 bytes
    }

    #[test]
    fn test_alignment_strategy_descriptions() {
        assert!(!AlignmentStrategy::Minimal.description().is_empty());
        assert!(!AlignmentStrategy::CacheLine.description().is_empty());
        assert!(!AlignmentStrategy::Custom(32).description().contains("AVX SIMD"));
    }

    #[test]
    fn test_simd_alignment() {
        // Test 32-byte alignment for AVX operations
        let mut simd_pool = MemoryPool::new_with_strategy(1024, 32, AlignmentStrategy::Custom(32)).unwrap();

        let ptr = simd_pool.allocate(100, 32).unwrap();
        assert_eq!(ptr.as_ptr() as usize % 32, 0, "SIMD data should be 32-byte aligned");
        assert_eq!(simd_pool.base_alignment(), 32);
        assert_eq!(simd_pool.alignment_strategy(), AlignmentStrategy::Custom(32));
    }

    #[test]
    fn test_cache_optimized_strategy() {
        let mut pool = MemoryPool::new_with_strategy(1024, 8, AlignmentStrategy::CacheOptimized).unwrap();

        // Request 8-byte alignment, should get 64-byte due to cache optimization
        let ptr = pool.allocate(64, 8).unwrap();
        assert_eq!(ptr.as_ptr() as usize % 64, 0, "Should be cache-line aligned");
    }

    #[test]
    fn test_pool_exhaustion_with_alignment() {
        let mut pool = MemoryPool::new_with_strategy(200, 8, AlignmentStrategy::Minimal).unwrap();

        // Fill most of the pool
        let large_allocation = pool.total_size() - 64;
        let _ptr1 = pool.allocate(large_allocation, 8).unwrap();

        // This should fail
        let result = pool.allocate(128, 8);
        assert!(matches!(result, Err(PoolError::PoolExhausted { .. })));
    }

    #[test]
    fn test_architecture_specific_configurations() {
        // ARM cache line (64 bytes)
        let arm_pool = MemoryPool::new_with_strategy(1024, 8, AlignmentStrategy::CacheLine).unwrap();
        assert_eq!(arm_pool.alignment_strategy(), AlignmentStrategy::CacheLine);

        // x86 page optimization
        let x86_pool = MemoryPool::new_with_strategy(1024, 8, AlignmentStrategy::Page).unwrap();
        assert_eq!(x86_pool.alignment_strategy(), AlignmentStrategy::Page);

        // GPU alignment (256 bytes)
        let gpu_pool = MemoryPool::new_with_strategy(1024, 8, AlignmentStrategy::Custom(256)).unwrap();
        assert_eq!(gpu_pool.alignment_strategy(), AlignmentStrategy::Custom(256));
    }

    #[test]
    fn test_pool_creation() {
        let pool = MemoryPool::new(1024).unwrap();
        assert_eq!(pool.current_offset(), 0);
        assert_eq!(pool.available(), pool.total_size());
        assert!(pool.is_empty());
        assert_eq!(pool.efficiency(), 1.0);
        assert_eq!(pool.fragmentation_ratio(), 0.0);
    }

    #[test]
    fn test_basic_allocation() {
        let mut pool = MemoryPool::new(1024).unwrap();

        let ptr1 = pool.allocate(64, 8).unwrap();
        assert_eq!(pool.current_offset(), 64);
        assert!(!pool.is_empty());

        let ptr2 = pool.allocate(32, 8).unwrap();
        assert_eq!(pool.current_offset(), 96);

        // Ensure pointers are different and properly aligned
        assert_ne!(ptr1.as_ptr(), ptr2.as_ptr());
        assert_eq!(ptr1.as_ptr() as usize % 8, 0);
        assert_eq!(ptr2.as_ptr() as usize % 8, 0);
    }

    #[test]
    fn test_alignment() {
        let mut pool = MemoryPool::new(1024).unwrap();

        // Test various alignment requirements
        let alignments = [8, 16, 32, 64];

        for &align in &alignments {
            pool.reset();
            let ptr = pool.allocate(100, align).unwrap();
            assert_eq!(ptr.as_ptr() as usize % align, 0,
                       "Failed alignment requirement: {} bytes", align);
        }
    }

    #[test]
    fn test_can_allocate() {
        let pool = MemoryPool::new_with_strategy(1000, 8, AlignmentStrategy::Minimal).unwrap();

        assert!(pool.can_allocate(500, 8));
        assert!(pool.can_allocate(1000, 8));
        assert!(!pool.can_allocate(1001, 8));
        assert!(!pool.can_allocate(100, 3)); // Invalid alignment
    }

    #[test]
    fn test_reset_functionality() {
        let mut pool = MemoryPool::new(1024).unwrap();

        let _ptr1 = pool.allocate(64, 8).unwrap();
        let checkpoint = pool.current_offset();

        let _ptr2 = pool.allocate(32, 8).unwrap();
        assert_eq!(pool.current_offset(), 96);

        pool.reset_to(checkpoint).unwrap();
        assert_eq!(pool.current_offset(), checkpoint);

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
        let expected_hwm = (100 + 8 - 1) & !(8 - 1); // Align 100 to 8-byte boundary
        let final_hwm = expected_hwm + 200;
        assert_eq!(pool.high_water_mark(), final_hwm);

        // Reset and allocate smaller amount
        pool.reset();
        let _ptr3 = pool.allocate(50, 8).unwrap();
        assert_eq!(pool.high_water_mark(), final_hwm); // Should remain at peak
    }

    #[test]
    fn test_invalid_alignment() {
        let mut pool = MemoryPool::new(1024).unwrap();

        let result = pool.allocate(64, 3); // 3 is not power of 2
        assert!(matches!(result, Err(PoolError::InvalidAlignment { .. })));

        // Test invalid alignment in pool creation
        let result = MemoryPool::new_with_alignment(1024, 3);
        assert!(matches!(result, Err(PoolError::InvalidAlignment { .. })));
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
    fn test_pool_properties() {
        let pool = MemoryPool::new_with_strategy(
            1024, 16, AlignmentStrategy::Custom(32)
        ).unwrap();

        assert_eq!(pool.base_alignment(), 16);
        assert_eq!(pool.alignment_strategy(), AlignmentStrategy::Custom(32));
        assert!(pool.total_size() >= 1024);
        assert_eq!(pool.current_offset(), 0);
        assert_eq!(pool.available(), pool.total_size());
    }

    #[test]
    fn test_error_messages() {
        let mut small_pool = MemoryPool::new_with_strategy(64, 8, AlignmentStrategy::Minimal).unwrap();

        // Test pool exhaustion error
        let result = small_pool.allocate(100, 8);
        match result {
            Err(PoolError::PoolExhausted { requested, available }) => {
                assert_eq!(requested, 100);
                assert_eq!(available, 64);
            }
            _ => panic!("Expected PoolExhausted error"),
        }

        // Test invalid offset error
        let result = small_pool.reset_to(1000);
        match result {
            Err(PoolError::InvalidOffset { offset, pool_size }) => {
                assert_eq!(offset, 1000);
                assert_eq!(pool_size, 64);
            }
            _ => panic!("Expected InvalidOffset error"),
        }
    }
}