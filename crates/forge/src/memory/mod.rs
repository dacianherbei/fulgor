//! High-performance memory pool system for stack-like heap allocation.
//!
//! This module provides memory pools that achieve O(1) allocation performance
//! for node-based execution workflows with architecture-aware alignment strategies.
//!
//! ## Key Features
//!
//! - **Architecture-Aware Alignment**: Optimized for x86_64, ARM64, RISC-V, and WebAssembly
//! - **Use-Case Specific Strategies**: SIMD, GPU, cache-optimized, and minimal overhead modes
//! - **Stack-Like Semantics**: O(1) allocation with checkpoint-based reset functionality
//! - **Performance Statistics**: Detailed tracking of allocation patterns and efficiency
//!
//! ## Examples
//!
//! ```rust
//! use forge::memory::{MemoryPool, AlignmentStrategy};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Architecture-aware pool
//! let mut pool = MemoryPool::new(1024 * 1024)?;
//!
//! // SIMD-optimized pool
//! let mut simd_pool = MemoryPool::new_with_strategy(
//!     1024 * 1024, 32, AlignmentStrategy::Custom(32)
//! )?;
//!
//! // Allocate with automatic alignment optimization
//! let ptr = pool.allocate(64, 8)?;
//!
//! // Stack-like reset
//! let checkpoint = pool.current_offset();
//! let temp_ptr = pool.allocate(256, 16)?;
//! pool.reset_to(checkpoint)?; // "Pop" the temporary allocation
//! # Ok(())
//! # }
//! ```

pub mod pool;
pub mod stats;

// Re-export for convenience when using memory module directly
pub use pool::{MemoryPool, PoolError, AlignmentStrategy};
pub use stats::{PoolStats, MemoryUtilization};