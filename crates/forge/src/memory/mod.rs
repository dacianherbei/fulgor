// File: crates/forge/src/memory/mod.rs
//! High-performance memory pool system for stack-like heap allocation.
//!
//! This module provides memory pools that achieve O(1) allocation performance
//! for node-based execution workflows.

pub mod pool;
pub mod stats;