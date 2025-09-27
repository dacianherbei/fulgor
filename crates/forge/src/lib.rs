//! # Forge - Memory Management and Code Generation
//!
//! Forge provides high-performance memory management and LLVM-based code generation
//! for the Fulgor IDE. The primary focus is on achieving stack-like allocation
//! performance for heap operations in node-based execution workflows.
//!
//! ## Core Features
//!
//! - **Memory Pools**: O(1) allocation with stack-like reset semantics
//! - **Architecture-Aware Alignment**: Optimized for different CPU architectures and use cases
//! - **Statistics Tracking**: Detailed performance and utilization metrics
//! - **LLVM Integration**: (Coming soon) Cross-language compilation support
//!
//! ## Architecture Support
//!
//! Forge automatically optimizes memory alignment for:
//! - **x86_64**: 64-byte cache lines, 4KB pages
//! - **ARM64**: 64-byte cache lines, mobile/server optimization
//! - **RISC-V**: 32-byte cache lines, embedded systems
//! - **WebAssembly**: Minimal overhead, compact alignment
//!
//! ## Quick Start
//!
//! ```rust
//! use forge::{ForgeConfig, TargetArchitecture, AlignmentStrategy};
//! use forge::memory::MemoryPool;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Architecture-specific configuration (automatic detection)
//! let config = ForgeConfig::for_architecture(TargetArchitecture::current());
//! let mut pool = config.create_pool()?;
//!
//! // Or create directly with alignment strategy
//! let mut simd_pool = MemoryPool::new_with_strategy(
//!     1024 * 1024, 32, AlignmentStrategy::Custom(32) // AVX alignment
//! )?;
//!
//! // Allocate memory with optimal alignment
//! let ptr1 = pool.allocate(64, 8)?;  // 64 bytes, 8-byte aligned
//! let ptr2 = pool.allocate(128, 16)?; // 128 bytes, 16-byte aligned
//!
//! println!("Pool usage: {} / {} bytes",
//!          pool.current_offset(), pool.total_size());
//!
//! // Stack-like memory management
//! let checkpoint = pool.current_offset();
//! let temp_ptr = pool.allocate(256, 32)?;
//! pool.reset_to(checkpoint)?; // "Pop" temporary allocation
//!
//! // Performance statistics
//! let stats = pool.get_stats();
//! println!("Efficiency: {:.1}%", pool.efficiency() * 100.0);
//! # Ok(())
//! # }
//! ```
//!
//! ## Use Case Examples
//!
//! ```rust
//! use forge::ForgeConfig;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Small, frequent allocations (L1 cache optimized)
//! let small_config = ForgeConfig::optimized_for_small_allocations();
//! let small_pool = small_config.create_pool()?;
//!
//! // SIMD operations (AVX, NEON)
//! let simd_config = ForgeConfig::optimized_for_simd(32); // 32-byte for AVX
//! let simd_pool = simd_config.create_pool()?;
//!
//! // GPU operations
//! let gpu_config = ForgeConfig::optimized_for_gpu();
//! let gpu_pool = gpu_config.create_pool()?;
//!
//! // Large data processing
//! let big_data_config = ForgeConfig::optimized_for_large_data();
//! let big_data_pool = big_data_config.create_pool()?;
//! # Ok(())
//! # }
//! ```

pub mod memory;

// Re-export commonly used types for convenience
pub use memory::pool::{MemoryPool, PoolError, AlignmentStrategy};
pub use memory::stats::{PoolStats, MemoryUtilization};

/// Version information for the forge crate
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Target architectures with different alignment requirements and optimizations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetArchitecture {
    /// x86_64 with 64-byte cache lines and 4KB pages
    /// Optimized for Intel/AMD processors
    X86_64,
    /// ARM64 with 64-byte cache lines
    /// Optimized for ARM Cortex-A and Apple Silicon
    ARM64,
    /// RISC-V with variable cache line sizes
    /// Optimized for RISC-V implementations
    RISCV,
    /// WebAssembly with minimal alignment requirements
    /// Optimized for web environments and minimal overhead
    WebAssembly,
}

impl TargetArchitecture {
    /// Detect the current target architecture at compile time
    ///
    /// This function uses compile-time target detection to automatically
    /// select the appropriate architecture configuration.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use forge::TargetArchitecture;
    ///
    /// let arch = TargetArchitecture::current();
    /// println!("Running on: {:?}", arch);
    /// ```
    pub fn current() -> Self {
        #[cfg(target_arch = "x86_64")]
        return TargetArchitecture::X86_64;

        #[cfg(target_arch = "aarch64")]
        return TargetArchitecture::ARM64;

        #[cfg(target_arch = "riscv64")]
        return TargetArchitecture::RISCV;

        #[cfg(target_arch = "wasm32")]
        return TargetArchitecture::WebAssembly;

        // Default fallback for other architectures
        #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64",
            target_arch = "riscv64", target_arch = "wasm32")))]
        return TargetArchitecture::X86_64;
    }

    /// Get the recommended cache line size for this architecture
    pub fn cache_line_size(&self) -> usize {
        match self {
            TargetArchitecture::X86_64 => 64,
            TargetArchitecture::ARM64 => 64,
            TargetArchitecture::RISCV => 32,
            TargetArchitecture::WebAssembly => 8,
        }
    }

    /// Get the recommended page size for this architecture
    pub fn page_size(&self) -> usize {
        match self {
            TargetArchitecture::X86_64 => 4096,
            TargetArchitecture::ARM64 => 4096,
            TargetArchitecture::RISCV => 4096,
            TargetArchitecture::WebAssembly => 64 * 1024, // 64KB WASM pages
        }
    }
}

/// Configuration for forge components with architecture-aware alignment strategies
///
/// ForgeConfig provides pre-configured setups for different use cases and architectures,
/// optimizing memory pool behavior for specific scenarios.
#[derive(Debug, Clone)]
pub struct ForgeConfig {
    /// Default memory pool size in bytes
    pub default_pool_size: usize,
    /// Default alignment for allocations
    pub default_alignment: usize,
    /// Alignment strategy for optimal performance
    pub alignment_strategy: AlignmentStrategy,
    /// Enable performance statistics collection
    pub enable_stats: bool,
}

impl Default for ForgeConfig {
    fn default() -> Self {
        Self {
            default_pool_size: 1024 * 1024, // 1MB
            default_alignment: 8,
            alignment_strategy: AlignmentStrategy::CacheOptimized,
            enable_stats: true,
        }
    }
}

impl ForgeConfig {
    /// Create a configuration optimized for small, frequent allocations.
    ///
    /// Uses cache line alignment for optimal L1 cache performance.
    /// Ideal for node-based execution with many small temporary objects.
    ///
    /// # Characteristics
    /// - Pool size: 64KB (fits in L1 cache)
    /// - Alignment: Cache line optimized
    /// - Use case: Frequent small allocations, low latency
    pub fn optimized_for_small_allocations() -> Self {
        Self {
            default_pool_size: 64 * 1024, // 64KB - fits in L1 cache
            default_alignment: 8,
            alignment_strategy: AlignmentStrategy::CacheLine,
            enable_stats: true,
        }
    }

    /// Create a configuration optimized for large data processing.
    ///
    /// Uses page alignment for optimal virtual memory performance.
    /// Ideal for processing large datasets or streaming operations.
    ///
    /// # Characteristics
    /// - Pool size: 16MB (large working set)
    /// - Alignment: Page boundary optimized
    /// - Use case: Large data processing, streaming
    pub fn optimized_for_large_data() -> Self {
        Self {
            default_pool_size: 16 * 1024 * 1024, // 16MB
            default_alignment: 64, // Cache line alignment
            alignment_strategy: AlignmentStrategy::Page,
            enable_stats: true,
        }
    }

    /// Create a configuration optimized for SIMD operations.
    ///
    /// Uses custom alignment for vector operations (AVX, NEON, etc.).
    ///
    /// # Arguments
    /// * `vector_width` - Width in bytes for SIMD vectors (16, 32, 64)
    ///
    /// # Examples
    /// ```rust
    /// use forge::ForgeConfig;
    ///
    /// // AVX 256-bit (32 bytes)
    /// let avx_config = ForgeConfig::optimized_for_simd(32);
    ///
    /// // AVX-512 (64 bytes)
    /// let avx512_config = ForgeConfig::optimized_for_simd(64);
    ///
    /// // NEON 128-bit (16 bytes)
    /// let neon_config = ForgeConfig::optimized_for_simd(16);
    /// ```
    pub fn optimized_for_simd(vector_width: usize) -> Self {
        Self {
            default_pool_size: 1024 * 1024, // 1MB
            default_alignment: vector_width,
            alignment_strategy: AlignmentStrategy::Custom(vector_width),
            enable_stats: true,
        }
    }

    /// Create a configuration optimized for GPU operations.
    ///
    /// Uses large alignment requirements typical for GPU memory transfers.
    /// Optimized for CUDA, OpenCL, and graphics operations.
    ///
    /// # Characteristics
    /// - Pool size: 32MB (GPU working set)
    /// - Alignment: 256-byte (GPU DMA alignment)
    /// - Use case: GPU buffers, texture data, compute shaders
    pub fn optimized_for_gpu() -> Self {
        Self {
            default_pool_size: 32 * 1024 * 1024, // 32MB
            default_alignment: 256, // GPU alignment requirement
            alignment_strategy: AlignmentStrategy::Custom(256),
            enable_stats: true,
        }
    }

    /// Create a configuration with minimal overhead.
    ///
    /// Uses only the requested alignment without additional padding.
    /// Ideal for memory-constrained environments.
    ///
    /// # Characteristics
    /// - Pool size: 64KB (minimal footprint)
    /// - Alignment: Minimal (no padding)
    /// - Statistics: Disabled (reduced overhead)
    /// - Use case: Embedded systems, WebAssembly
    pub fn minimal_overhead() -> Self {
        Self {
            default_pool_size: 64 * 1024, // 64KB
            default_alignment: 8,
            alignment_strategy: AlignmentStrategy::Minimal,
            enable_stats: false, // Disable stats for minimal overhead
        }
    }

    /// Create architecture-specific configurations
    ///
    /// Automatically configures optimal settings based on the target architecture's
    /// characteristics like cache line size, page size, and performance characteristics.
    ///
    /// # Arguments
    /// * `arch` - Target architecture to optimize for
    ///
    /// # Examples
    /// ```rust
    /// use forge::{ForgeConfig, TargetArchitecture};
    ///
    /// // Automatic detection
    /// let config = ForgeConfig::for_architecture(TargetArchitecture::current());
    ///
    /// // Specific architecture
    /// let x86_config = ForgeConfig::for_architecture(TargetArchitecture::X86_64);
    /// let arm_config = ForgeConfig::for_architecture(TargetArchitecture::ARM64);
    /// ```
    pub fn for_architecture(arch: TargetArchitecture) -> Self {
        match arch {
            TargetArchitecture::X86_64 => Self {
                default_pool_size: 2 * 1024 * 1024, // 2MB
                default_alignment: 64, // Cache line
                alignment_strategy: AlignmentStrategy::CacheOptimized,
                enable_stats: true,
            },
            TargetArchitecture::ARM64 => Self {
                default_pool_size: 1024 * 1024, // 1MB
                default_alignment: 64, // ARM cache line
                alignment_strategy: AlignmentStrategy::CacheLine,
                enable_stats: true,
            },
            TargetArchitecture::RISCV => Self {
                default_pool_size: 512 * 1024, // 512KB
                default_alignment: 32, // RISC-V cache line
                alignment_strategy: AlignmentStrategy::Custom(32),
                enable_stats: true,
            },
            TargetArchitecture::WebAssembly => Self {
                default_pool_size: 256 * 1024, // 256KB
                default_alignment: 8, // WASM alignment
                alignment_strategy: AlignmentStrategy::Minimal,
                enable_stats: false, // Minimize overhead in WASM
            },
        }
    }

    /// Create a memory pool using this configuration
    ///
    /// # Examples
    /// ```rust
    /// use forge::ForgeConfig;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let config = ForgeConfig::optimized_for_small_allocations();
    /// let pool = config.create_pool()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn create_pool(&self) -> Result<MemoryPool, PoolError> {
        MemoryPool::new_with_strategy(
            self.default_pool_size,
            self.default_alignment,
            self.alignment_strategy
        )
    }

    /// Create a memory pool with a specific size using this configuration's alignment strategy
    ///
    /// # Arguments
    /// * `size` - Size of the memory pool in bytes
    ///
    /// # Examples
    /// ```rust
    /// use forge::ForgeConfig;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let config = ForgeConfig::optimized_for_gpu();
    /// let pool = config.create_pool_with_size(64 * 1024 * 1024)?; // 64MB pool
    /// # Ok(())
    /// # }
    /// ```
    pub fn create_pool_with_size(&self, size: usize) -> Result<MemoryPool, PoolError> {
        MemoryPool::new_with_strategy(
            size,
            self.default_alignment,
            self.alignment_strategy
        )
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
        assert_eq!(config.alignment_strategy, AlignmentStrategy::CacheOptimized);
        assert!(config.enable_stats);
    }

    #[test]
    fn test_architecture_specific_configs() {
        let x86_config = ForgeConfig::for_architecture(TargetArchitecture::X86_64);
        assert_eq!(x86_config.default_alignment, 64);
        assert_eq!(x86_config.alignment_strategy, AlignmentStrategy::CacheOptimized);

        let arm_config = ForgeConfig::for_architecture(TargetArchitecture::ARM64);
        assert_eq!(arm_config.default_alignment, 64);
        assert_eq!(arm_config.alignment_strategy, AlignmentStrategy::CacheLine);

        let wasm_config = ForgeConfig::for_architecture(TargetArchitecture::WebAssembly);
        assert_eq!(wasm_config.default_alignment, 8);
        assert_eq!(wasm_config.alignment_strategy, AlignmentStrategy::Minimal);
    }

    #[test]
    fn test_simd_optimized_config() {
        let avx_config = ForgeConfig::optimized_for_simd(32); // AVX 256-bit
        assert_eq!(avx_config.default_alignment, 32);
        assert_eq!(avx_config.alignment_strategy, AlignmentStrategy::Custom(32));

        let avx512_config = ForgeConfig::optimized_for_simd(64); // AVX-512
        assert_eq!(avx512_config.default_alignment, 64);
        assert_eq!(avx512_config.alignment_strategy, AlignmentStrategy::Custom(64));
    }

    #[test]
    fn test_gpu_optimized_config() {
        let gpu_config = ForgeConfig::optimized_for_gpu();
        assert_eq!(gpu_config.default_alignment, 256);
        assert_eq!(gpu_config.alignment_strategy, AlignmentStrategy::Custom(256));
        assert_eq!(gpu_config.default_pool_size, 32 * 1024 * 1024);
    }

    #[test]
    fn test_minimal_overhead_config() {
        let minimal_config = ForgeConfig::minimal_overhead();
        assert_eq!(minimal_config.alignment_strategy, AlignmentStrategy::Minimal);
        assert!(!minimal_config.enable_stats); // Stats disabled for minimal overhead
    }

    #[test]
    fn test_current_architecture_detection() {
        let current_arch = TargetArchitecture::current();
        let config = ForgeConfig::for_architecture(current_arch);

        // Should create a valid pool for the current architecture
        let pool = config.create_pool().unwrap();
        assert!(pool.total_size() > 0);
    }

    #[test]
    fn test_architecture_properties() {
        assert_eq!(TargetArchitecture::X86_64.cache_line_size(), 64);
        assert_eq!(TargetArchitecture::ARM64.cache_line_size(), 64);
        assert_eq!(TargetArchitecture::RISCV.cache_line_size(), 32);
        assert_eq!(TargetArchitecture::WebAssembly.cache_line_size(), 8);

        assert_eq!(TargetArchitecture::X86_64.page_size(), 4096);
        assert_eq!(TargetArchitecture::WebAssembly.page_size(), 64 * 1024);
    }

    #[test]
    fn test_config_pool_creation() {
        let config = ForgeConfig::optimized_for_small_allocations();
        let pool = config.create_pool().unwrap();
        assert!(pool.total_size() >= 64 * 1024);
        assert_eq!(pool.alignment_strategy(), AlignmentStrategy::CacheLine);
    }

    #[test]
    fn test_config_pool_creation_with_size() {
        let config = ForgeConfig::optimized_for_gpu();
        let pool = config.create_pool_with_size(1024 * 1024).unwrap(); // 1MB
        assert!(pool.total_size() >= 1024 * 1024);
        assert_eq!(pool.alignment_strategy(), AlignmentStrategy::Custom(256));
    }

    #[test]
    fn test_version_constant() {
        assert!(!VERSION.is_empty());
    }
}