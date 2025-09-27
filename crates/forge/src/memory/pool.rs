use std::alloc::{GlobalAlloc, Layout, System};
use std::ptr::NonNull;
use anyhow::{Result};
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

/// High-performance memory pool with stack-like allocation semantics.
///
/// Provides O(1) allocation and stack-like deallocation through reset operations.
/// Designed for predictable, high-frequency allocation patterns in node execution.
pub struct MemoryPool {
    /// Pointer to the allocated buffer
    buffer: NonNull<u8>,
    /// Total size of the pool in bytes
    total_size: usize,
    /// Current allocation offset (stack pointer equivalent)
    current_offset: usize,
    /// Highest offset reached (high water mark)
    high_water_mark: usize,
    /// Required alignment for the pool
    alignment: usize,
    /// Layout used for the initial allocation
    layout: Layout,
    /// Statistics tracking
    stats: PoolStats,
}

impl MemoryPool {
    /// Create a new memory pool with the specified size.
    ///
    /// # Arguments
    /// * `size` - Minimum size of the pool in bytes. The actual size will be aligned
    ///           to 4KB page boundaries for optimal performance and may be larger.
    ///
    /// # Returns
    /// * `Ok(MemoryPool)` - Successfully created pool
    /// * `Err(PoolError)` - Failed to allocate backing memory
    ///
    /// # Examples
    /// ```
    /// # use forge::memory::pool::MemoryPool;
    /// let pool = MemoryPool::new(1000).unwrap();
    /// // Pool size will be at least 1000 bytes, but likely 4096 due to page alignment
    /// assert!(pool.total_size() >= 1000);
    /// ```
    pub fn new(size: usize) -> Result<Self, PoolError> {
        Self::new_with_alignment(size, 8) // Default to 8-byte alignment
    }

    /// Create a new memory pool with specific alignment requirements.
    ///
    /// # Arguments
    /// * `size` - Minimum size of the pool in bytes. The actual size will be aligned
    ///           to 4KB page boundaries for optimal performance and may be larger.
    /// * `alignment` - Required alignment (must be power of 2, minimum 8 bytes)
    ///
    /// # Examples
    /// ```
    /// # use forge::memory::pool::MemoryPool;
    /// let pool = MemoryPool::new_with_alignment(1000, 16).unwrap();
    /// assert!(pool.total_size() >= 1000);
    /// ```
    pub fn new_with_alignment(size: usize, alignment: usize) -> Result<Self, PoolError> {
        // Validate alignment is power of 2
        if !alignment.is_power_of_two() {
            return Err(PoolError::InvalidAlignment { alignment });
        }

        // Ensure minimum alignment of 8 bytes
        let alignment = alignment.max(8);

        // Align size to page boundaries for better performance
        // This reduces TLB misses and improves cache locality
        let aligned_size = (size + 4095) & !4095; // Round up to 4KB pages

        // Create layout for the buffer
        let layout = Layout::from_size_align(aligned_size, alignment)
            .map_err(|e| PoolError::AllocationFailed(format!("Invalid layout: {}", e)))?;

        // Allocate the backing buffer
        let buffer = unsafe {
            let ptr = System.alloc(layout);
            if ptr.is_null() {
                return Err(PoolError::AllocationFailed(
                    format!("Failed to allocate {} bytes", aligned_size)
                ));
            }
            NonNull::new_unchecked(ptr)
        };

        Ok(Self {
            buffer,
            total_size: aligned_size,
            current_offset: 0,
            high_water_mark: 0,
            alignment,
            layout,
            stats: PoolStats::new(),
        })
    }

    /// Allocate memory from the pool with specified size and alignment.
    ///
    /// # Arguments
    /// * `size` - Number of bytes to allocate
    /// * `align` - Alignment requirement (must be power of 2)
    ///
    /// # Returns
    /// * `Ok(NonNull<u8>)` - Pointer to allocated memory
    /// * `Err(PoolError)` - Allocation failed
    pub fn allocate(&mut self, size: usize, align: usize) -> Result<NonNull<u8>, PoolError> {
        // Validate alignment
        if !align.is_power_of_two() {
            return Err(PoolError::InvalidAlignment { alignment: align });
        }

        // Calculate aligned offset
        let aligned_offset = (self.current_offset + align - 1) & !(align - 1);
        let end_offset = aligned_offset + size;

        // Check if allocation fits
        if end_offset > self.total_size {
            return Err(PoolError::PoolExhausted {
                requested: size,
                available: self.total_size - aligned_offset,
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
        self.stats.record_allocation(size, align);

        Ok(ptr)
    }

    /// Reset the pool to a previous offset (stack-like "pop" operation).
    ///
    /// # Arguments
    /// * `offset` - Offset to reset to (must be <= current_offset)
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
    pub fn reset(&mut self) {
        self.current_offset = 0;
        self.stats.record_reset(0);
    }

    /// Get the current allocation offset (equivalent to stack pointer).
    pub fn current_offset(&self) -> usize {
        self.current_offset
    }

    /// Get the total size of the pool.
    pub fn total_size(&self) -> usize {
        self.total_size
    }

    /// Get the amount of memory currently available for allocation.
    pub fn available(&self) -> usize {
        self.total_size - self.current_offset
    }

    /// Get the high water mark (maximum offset reached).
    pub fn high_water_mark(&self) -> usize {
        self.high_water_mark
    }

    /// Get pool statistics.
    pub fn get_stats(&self) -> &PoolStats {
        &self.stats
    }

    /// Check if the pool is empty (no current allocations).
    pub fn is_empty(&self) -> bool {
        self.current_offset == 0
    }

    /// Get pool efficiency (percentage of allocated memory actually used).
    pub fn efficiency(&self) -> f32 {
        if self.high_water_mark == 0 {
            1.0
        } else {
            self.high_water_mark as f32 / self.total_size as f32
        }
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
unsafe impl Send for MemoryPool {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_page_alignment_behavior() {
        // Pool sizes get aligned to 4KB page boundaries for performance
        let small_pool = MemoryPool::new(100).unwrap();
        assert_eq!(small_pool.total_size(), 4096); // Aligned to 4KB

        let medium_pool = MemoryPool::new(5000).unwrap();
        assert_eq!(medium_pool.total_size(), 8192); // Aligned to 8KB

        let large_pool = MemoryPool::new(8192).unwrap();
        assert_eq!(large_pool.total_size(), 8192); // Already aligned
    }

    #[test]
    fn test_pool_creation() {
        let pool = MemoryPool::new(1024).unwrap();
        assert_eq!(pool.current_offset(), 0);
        assert_eq!(pool.available(), pool.total_size());
        assert!(pool.is_empty());
    }

    #[test]
    fn test_basic_allocation() {
        let mut pool = MemoryPool::new(1024).unwrap();

        let ptr1 = pool.allocate(64, 8).unwrap();
        assert_eq!(pool.current_offset(), 64);
        assert!(!pool.is_empty());

        let ptr2 = pool.allocate(32, 8).unwrap();
        assert_eq!(pool.current_offset(), 96);

        // Ensure pointers are different
        assert_ne!(ptr1.as_ptr(), ptr2.as_ptr());
    }

    #[test]
    fn test_alignment() {
        let mut pool = MemoryPool::new(1024).unwrap();

        // Allocate with 16-byte alignment
        let ptr = pool.allocate(10, 16).unwrap();
        assert_eq!(ptr.as_ptr() as usize % 16, 0);
    }

    #[test]
    fn test_pool_exhaustion() {
        let mut pool = MemoryPool::new(100).unwrap();

        // Get the actual pool size (may be aligned up to page boundaries)
        let actual_size = pool.total_size();

        // Fill most of the pool
        let large_allocation = actual_size - 64; // Leave small amount
        let _ptr1 = pool.allocate(large_allocation, 8).unwrap();

        // This should fail - trying to allocate more than remaining space
        let result = pool.allocate(128, 8); // More than the 64 bytes left
        assert!(matches!(result, Err(PoolError::PoolExhausted { .. })));
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
    fn test_invalid_alignment() {
        let mut pool = MemoryPool::new(1024).unwrap();

        let result = pool.allocate(64, 3); // 3 is not power of 2
        assert!(matches!(result, Err(PoolError::InvalidAlignment { .. })));
    }
}