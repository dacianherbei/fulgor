//! High-performance memory pool system for stack-like heap allocation.
//!
//! This module provides memory pools that achieve O(1) allocation performance
//! for node-based execution workflows.

pub mod pool;
pub mod stats;

pub use pool::{MemoryPool, PoolError};
pub use stats::PoolStats;