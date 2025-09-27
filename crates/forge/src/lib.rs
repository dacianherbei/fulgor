//! # Forge - Memory Management and Code Generation
//!
//! Forge provides high-performance memory management and LLVM-based code generation
//! for the Fulgor IDE. The primary focus is on achieving stack-like allocation
//! performance for heap operations in node-based execution workflows.
//!
//! ## Core Features
//!
//! - **Memory Pools**: O(1) allocation with stack-like reset semantics
//! - **Statistics Tracking**: Detailed performance and utilization metrics
//! - **LLVM Integration**: (Coming soon) Cross-language compilation support
//!
//! ## Quick Start
//!
//! ```rust
//! use forge::memory::pool::MemoryPool;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Create a 1MB memory pool
//! let mut pool = MemoryPool::new(1024 * 1024)?;
//!
//! // Allocate some memory
//! let ptr1 = pool.allocate(64, 8)?;  // 64 bytes, 8-byte aligned
//! let ptr2 = pool.allocate(128, 16)?; // 128 bytes, 16-byte aligned
//!
//! println!("Pool usage: {} / {} bytes", 
//!          pool.current_offset(), pool.total_size());
//!
//! // Reset pool (stack-like "pop")
//! let checkpoint = pool.current_offset();
//! let ptr3 = pool.allocate(256, 32)?;
//! pool.reset_to(checkpoint)?; // "Free" ptr3
//!
//! // Get performance statistics
//! let stats = pool.get_stats();
//! println!("Allocations: {}", stats.total_allocations);
//! println!("Efficiency: {:.1}%", pool.efficiency() * 100.0);
//! # Ok(())
//! # }
//! ```

pub mod memory;

// Re-export commonly used types for convenience
pub use memory::pool::{MemoryPool, PoolError};
pub use memory::stats::PoolStats;

/// Version information for the forge crate
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Configuration for forge components
#[derive(Debug, Clone)]
pub struct ForgeConfig {
    /// Default memory pool size in bytes
    pub default_pool_size: usize,
    /// Default alignment for allocations
    pub default_alignment: usize,
    /// Enable performance statistics collection
    pub enable_stats: bool,
}

impl Default for ForgeConfig {
    fn default() -> Self {
        Self {
            default_pool_size: 1024 * 1024, // 1MB
            default_alignment: 8,
            enable_stats: true,
        }
    }
}

impl ForgeConfig {
    /// Create a configuration optimized for small, frequent allocations
    pub fn optimized_for_small_allocations() -> Self {
        Self {
            default_pool_size: 64 * 1024, // 64KB - fits in L1 cache
            default_alignment: 8,
            enable_stats: true,
        }
    }

    /// Create a configuration optimized for large data processing
    pub fn optimized_for_large_data() -> Self {
        Self {
            default_pool_size: 16 * 1024 * 1024, // 16MB
            default_alignment: 64, // Cache line alignment
            enable_stats: true,
        }
    }

    /// Create a memory pool using this configuration
    pub fn create_pool(&self) -> Result<MemoryPool, PoolError> {
        MemoryPool::new_with_alignment(self.default_pool_size, self.default_alignment)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_forge_config_defaults() {
        let config = ForgeConfig::default();
        assert_eq!(config.default_pool_size, 1024 * 1024);
        assert_eq!(config.default_alignment, 8);
        assert!(config.enable_stats);
    }

    #[test]
    fn test_config_pool_creation() {
        let config = ForgeConfig::optimized_for_small_allocations();
        let pool = config.create_pool().unwrap();
        assert_eq!(pool.total_size(), 65536); // Should be aligned to page boundary
    }

    #[test]
    fn test_version_constant() {
        assert!(!VERSION.is_empty());
    }
}