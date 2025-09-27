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
//! - **Research-Based Optimizations**: Ultra-fast pools based on recent memory allocation research
//!
//! ## Pool Types
//!
//! This module provides two types of memory pools:
//!
//! - **`MemoryPool`**: Full-featured pool with comprehensive statistics and safety checks
//! - **`OptimizedPool`**: Ultra-fast pool with research-based optimizations for maximum speed
//!
//! ## Examples
//!
//! ```rust
//! use forge::memory::{MemoryPool, AlignmentStrategy};
//! use forge::memory::optimized_pool::{OptimizedPool, FastPoolConfig};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Standard pool with full features
//! let mut pool = MemoryPool::new(1024 * 1024)?;
//! let ptr1 = pool.allocate(64, 8)?;
//!
//! // Ultra-fast optimized pool
//! let config = FastPoolConfig::fastest();
//! let mut fast_pool = config.create_pool()?;
//! let ptr2 = fast_pool.allocate_fast(64)?;
//!
//! // SIMD-optimized pool
//! let mut simd_pool = MemoryPool::new_with_strategy(
//!     1024 * 1024, 32, AlignmentStrategy::Custom(32)
//! )?;
//! let simd_ptr = simd_pool.allocate(256, 32)?; // AVX-aligned
//!
//! // Stack-like reset (ultra-fast)
//! fast_pool.reset_fast();
//! # Ok(())
//! # }
//! ```
//!
//! ## Performance Comparison
//!
//! Based on research findings, the optimized pool can achieve:
//! - **3x faster allocation** than malloc in ideal conditions
//! - **100x faster deallocation** through stack-like reset
//! - **Zero overhead statistics** when disabled
//! - **CPU prefetching optimizations** for better cache performance
//!
//! ## When to Use Which Pool
//!
//! - **Use `MemoryPool`** for: Development, debugging, comprehensive statistics, safety
//! - **Use `OptimizedPool`** for: Production hot paths, maximum performance, minimal overhead

pub mod pool;
pub mod stats;
pub mod optimized_pool;

// Re-export for convenience when using memory module directly
pub use pool::{MemoryPool, PoolError, AlignmentStrategy};
pub use stats::{PoolStats, MemoryUtilization};
pub use optimized_pool::{OptimizedPool, FastPoolConfig};