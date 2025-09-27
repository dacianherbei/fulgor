// File: crates/forge/src/memory/stats.rs
use std::time::{Duration, Instant};

/// Performance statistics for memory pool operations.
///
/// Tracks allocation patterns, efficiency metrics, and performance data
/// to help optimize memory usage patterns.
#[derive(Debug, Clone)]
pub struct PoolStats {
    /// Total number of allocation requests
    pub total_allocations: u64,
    /// Total bytes allocated across all requests
    pub total_bytes_allocated: u64,
    /// Number of pool resets performed
    pub reset_count: u64,
    /// Creation time of the pool
    pub creation_time: Instant,
    /// Average allocation size
    pub average_allocation_size: f64,
    /// Largest single allocation size
    pub largest_allocation: usize,
    /// Smallest single allocation size
    pub smallest_allocation: usize,
    /// Most common alignment requirements
    pub alignment_histogram: [u64; 8], // Powers of 2: 1, 2, 4, 8, 16, 32, 64, 128+
    /// Peak memory usage (high water mark tracking)
    pub peak_usage_bytes: usize,
    /// Total time spent in allocation operations (for performance analysis)
    pub total_allocation_time: Duration,
}

impl PoolStats {
    /// Create a new statistics tracker.
    pub fn new() -> Self {
        Self {
            total_allocations: 0,
            total_bytes_allocated: 0,
            reset_count: 0,
            creation_time: Instant::now(),
            average_allocation_size: 0.0,
            largest_allocation: 0,
            smallest_allocation: usize::MAX,
            alignment_histogram: [0; 8],
            peak_usage_bytes: 0,
            total_allocation_time: Duration::ZERO,
        }
    }

    /// Record a memory allocation.
    pub fn record_allocation(&mut self, size: usize, alignment: usize) {
        let start_time = Instant::now();

        self.total_allocations += 1;
        self.total_bytes_allocated += size as u64;

        // Update size statistics
        self.largest_allocation = self.largest_allocation.max(size);
        if size > 0 {
            self.smallest_allocation = self.smallest_allocation.min(size);
        }

        // Update average allocation size
        self.average_allocation_size =
            self.total_bytes_allocated as f64 / self.total_allocations as f64;

        // Update alignment histogram
        let alignment_index = self.alignment_to_index(alignment);
        self.alignment_histogram[alignment_index] += 1;

        // Record allocation time (in a real implementation, this would be more sophisticated)
        self.total_allocation_time += start_time.elapsed();
    }

    /// Record a pool reset operation.
    pub fn record_reset(&mut self, new_offset: usize) {
        self.reset_count += 1;
        self.peak_usage_bytes = self.peak_usage_bytes.max(new_offset);
    }

    /// Get the pool efficiency as a percentage (0.0 to 1.0).
    ///
    /// Efficiency is calculated as the ratio of actually allocated bytes
    /// to the total memory that was reserved for allocations.
    pub fn efficiency(&self, pool_total_size: usize) -> f32 {
        if pool_total_size == 0 {
            1.0
        } else {
            self.peak_usage_bytes as f32 / pool_total_size as f32
        }
    }

    /// Get the allocation rate (allocations per second).
    pub fn allocation_rate(&self) -> f64 {
        let elapsed = self.creation_time.elapsed().as_secs_f64();
        if elapsed > 0.0 {
            self.total_allocations as f64 / elapsed
        } else {
            0.0
        }
    }

    /// Get the average allocation time in nanoseconds.
    pub fn average_allocation_time_ns(&self) -> f64 {
        if self.total_allocations > 0 {
            self.total_allocation_time.as_nanos() as f64 / self.total_allocations as f64
        } else {
            0.0
        }
    }

    /// Get the most commonly used alignment.
    pub fn most_common_alignment(&self) -> usize {
        let (max_index, _) = self.alignment_histogram
            .iter()
            .enumerate()
            .max_by_key(|(_, &count)| count)
            .unwrap_or((3, &0)); // Default to 8-byte alignment (index 3)

        self.index_to_alignment(max_index)
    }

    /// Get memory utilization statistics.
    pub fn memory_utilization(&self) -> MemoryUtilization {
        MemoryUtilization {
            total_allocated_bytes: self.total_bytes_allocated,
            peak_usage_bytes: self.peak_usage_bytes,
            average_allocation_size: self.average_allocation_size,
            largest_allocation: self.largest_allocation,
            smallest_allocation: if self.smallest_allocation == usize::MAX { 0 } else { self.smallest_allocation },
            fragmentation_ratio: self.calculate_fragmentation_ratio(),
        }
    }

    /// Convert alignment value to histogram index.
    fn alignment_to_index(&self, alignment: usize) -> usize {
        match alignment {
            1 => 0,
            2 => 1,
            4 => 2,
            8 => 3,
            16 => 4,
            32 => 5,
            64 => 6,
            _ => 7, // 128 or higher
        }
    }

    /// Convert histogram index back to alignment value.
    fn index_to_alignment(&self, index: usize) -> usize {
        match index {
            0 => 1,
            1 => 2,
            2 => 4,
            3 => 8,
            4 => 16,
            5 => 32,
            6 => 64,
            _ => 128,
        }
    }

    /// Calculate fragmentation ratio (simplified metric).
    fn calculate_fragmentation_ratio(&self) -> f32 {
        if self.total_allocations == 0 {
            0.0
        } else {
            // Simple fragmentation estimate: variance in allocation sizes
            let variance = if self.total_allocations > 1 {
                let size_variance = (self.largest_allocation - self.smallest_allocation) as f32;
                size_variance / self.average_allocation_size as f32
            } else {
                0.0
            };
            variance.min(1.0) // Cap at 1.0
        }
    }

    /// Reset all statistics (useful for benchmarking).
    pub fn reset_stats(&mut self) {
        *self = Self::new();
    }
}

impl Default for PoolStats {
    fn default() -> Self {
        Self::new()
    }
}

/// Detailed memory utilization information.
#[derive(Debug, Clone)]
pub struct MemoryUtilization {
    pub total_allocated_bytes: u64,
    pub peak_usage_bytes: usize,
    pub average_allocation_size: f64,
    pub largest_allocation: usize,
    pub smallest_allocation: usize,
    pub fragmentation_ratio: f32,
}

impl MemoryUtilization {
    /// Format utilization information as a human-readable string.
    pub fn format_summary(&self) -> String {
        format!(
            "Memory Utilization:\n\
             - Total allocated: {} bytes\n\
             - Peak usage: {} bytes\n\
             - Average allocation: {:.1} bytes\n\
             - Size range: {} - {} bytes\n\
             - Fragmentation: {:.1}%",
            self.total_allocated_bytes,
            self.peak_usage_bytes,
            self.average_allocation_size,
            self.smallest_allocation,
            self.largest_allocation,
            self.fragmentation_ratio * 100.0
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stats_creation() {
        let stats = PoolStats::new();
        assert_eq!(stats.total_allocations, 0);
        assert_eq!(stats.total_bytes_allocated, 0);
        assert_eq!(stats.reset_count, 0);
    }

    #[test]
    fn test_allocation_recording() {
        let mut stats = PoolStats::new();

        stats.record_allocation(64, 8);
        assert_eq!(stats.total_allocations, 1);
        assert_eq!(stats.total_bytes_allocated, 64);
        assert_eq!(stats.largest_allocation, 64);
        assert_eq!(stats.smallest_allocation, 64);
        assert_eq!(stats.average_allocation_size, 64.0);

        stats.record_allocation(32, 16);
        assert_eq!(stats.total_allocations, 2);
        assert_eq!(stats.total_bytes_allocated, 96);
        assert_eq!(stats.average_allocation_size, 48.0);
    }

    #[test]
    fn test_alignment_histogram() {
        let mut stats = PoolStats::new();

        stats.record_allocation(64, 8);
        stats.record_allocation(32, 8);
        stats.record_allocation(16, 16);

        assert_eq!(stats.alignment_histogram[3], 2); // 8-byte alignment
        assert_eq!(stats.alignment_histogram[4], 1); // 16-byte alignment
        assert_eq!(stats.most_common_alignment(), 8);
    }

    #[test]
    fn test_memory_utilization() {
        let mut stats = PoolStats::new();

        stats.record_allocation(100, 8);
        stats.record_allocation(200, 8);
        stats.record_reset(300);

        let utilization = stats.memory_utilization();
        assert_eq!(utilization.total_allocated_bytes, 300);
        assert_eq!(utilization.peak_usage_bytes, 300);
        assert_eq!(utilization.average_allocation_size, 150.0);
        assert_eq!(utilization.largest_allocation, 200);
        assert_eq!(utilization.smallest_allocation, 100);
    }
}