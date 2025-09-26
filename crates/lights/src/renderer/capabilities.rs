//! Capability system for the fulgor renderer.
//!
//! This module provides the foundational capability system that allows
//! different renderer components to expose their features and characteristics
//! in a consistent, discoverable manner.

pub use crate::renderer::{Capability, ProcessingUnitCapability};

/// Standard capability names used throughout the renderer system.
pub mod standard {
    /// Basic rendering capability
    pub const RENDERING: &str = "rendering";

    /// GPU acceleration capability
    pub const GPU_ACCELERATION: &str = "gpu_acceleration";

    /// CPU-based rendering capability
    pub const CPU_RENDERING: &str = "cpu_rendering";

    /// Real-time rendering capability
    pub const REALTIME: &str = "realtime";

    /// High-precision computation capability
    pub const HIGH_PRECISION: &str = "high_precision";

    /// Memory-efficient operations capability
    pub const MEMORY_EFFICIENT: &str = "memory_efficient";

    /// Tile-based rendering capability
    pub const TILE_BASED: &str = "tile_based";

    /// 3D Gaussian splatting capability
    pub const GAUSSIAN_SPLATTING: &str = "gaussian_splatting";
}

/// Helper functions for working with capabilities.
pub mod helpers {
    use crate::renderer::DataPrecision;

    /// Check if a precision is considered "high precision".
    pub fn is_high_precision(precision: DataPrecision) -> bool {
        matches!(precision, DataPrecision::F64)
    }

    /// Check if a precision is considered "memory efficient".
    pub fn is_memory_efficient(precision: DataPrecision) -> bool {
        matches!(precision, DataPrecision::F16 | DataPrecision::BFloat16)
    }

    /// Get the memory usage factor relative to F32.
    pub fn memory_usage_factor(precision: DataPrecision) -> f32 {
        match precision {
            DataPrecision::F16 | DataPrecision::BFloat16 => 0.5,
            DataPrecision::F32 => 1.0,
            DataPrecision::F64 => 2.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::helpers::*;
    use crate::renderer::DataPrecision;

    #[test]
    fn test_precision_classification() {
        assert!(is_high_precision(DataPrecision::F64));
        assert!(!is_high_precision(DataPrecision::F32));
        assert!(!is_high_precision(DataPrecision::F16));
        assert!(!is_high_precision(DataPrecision::BFloat16));
    }

    #[test]
    fn test_memory_efficiency() {
        assert!(is_memory_efficient(DataPrecision::F16));
        assert!(is_memory_efficient(DataPrecision::BFloat16));
        assert!(!is_memory_efficient(DataPrecision::F32));
        assert!(!is_memory_efficient(DataPrecision::F64));
    }

    #[test]
    fn test_memory_usage_factors() {
        assert_eq!(memory_usage_factor(DataPrecision::F16), 0.5);
        assert_eq!(memory_usage_factor(DataPrecision::BFloat16), 0.5);
        assert_eq!(memory_usage_factor(DataPrecision::F32), 1.0);
        assert_eq!(memory_usage_factor(DataPrecision::F64), 2.0);
    }
}