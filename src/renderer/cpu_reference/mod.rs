//! CPU reference renderer for the fulgor library
//!
//! This module provides a reference implementation of a CPU-based renderer
//! for 3D Gaussian Splatting primitives. It serves as a baseline implementation
//! for testing and development purposes.

use crate::renderer::Renderer;
use std::ops::{Add, AddAssign};

/// A CPU-based reference renderer for 3D Gaussian Splatting.
///
/// This renderer provides a template-able implementation that can work with
/// different precision levels. The template parameter `T` represents the
/// numeric type used for internal calculations.
///
/// # Type Parameters
///
/// * `T` - The numeric type for calculations (typically `f32` or `f64`)
///
/// # Examples
///
/// ```rust
/// use fulgor::renderer::cpu_reference::CpuReferenceRenderer;
///
/// // Create a single-precision renderer
/// let mut renderer_f32: CpuReferenceRenderer<f32> = CpuReferenceRenderer::new();
///
/// // Create a double-precision renderer
/// let mut renderer_f64: CpuReferenceRenderer<f64> = CpuReferenceRenderer::new();
/// ```
#[derive(Debug, Clone)]
pub struct CpuReferenceRenderer<NumberType = f32>
where
    NumberType: Copy + Default + PartialEq + Add<Output = NumberType> + AddAssign + Send + Sync,
{
    /// Indicates whether the renderer is currently running
    pub is_running: bool,

    /// Total number of frames rendered since creation or last reset
    pub frame_count: u64,

    /// Phantom data to maintain the template parameter
    _phantom: std::marker::PhantomData<NumberType>,
}

impl<NumberType> CpuReferenceRenderer<NumberType>
where
    NumberType: Copy + Default + PartialEq + Add<Output = NumberType> + AddAssign + Send + Sync,
{
    /// Creates a new `CpuReferenceRenderer` instance.
    ///
    /// The renderer starts in a stopped state with zero frames rendered.
    ///
    /// # Returns
    ///
    /// A new `CpuReferenceRenderer` with:
    /// - `is_running`: `false`
    /// - `frame_count`: `0`
    ///
    /// # Examples
    ///
    /// ```rust
    /// use fulgor::renderer::cpu_reference::CpuReferenceRenderer;
    ///
    /// let renderer: CpuReferenceRenderer<f32> = CpuReferenceRenderer::new();
    /// assert_eq!(renderer.is_running, false);
    /// assert_eq!(renderer.frame_count, 0);
    /// ```
    pub fn new() -> Self {
        Self {
            is_running: false,
            frame_count: 0,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Returns the current frame count.
    ///
    /// This method provides access to the total number of frames
    /// that have been rendered since the renderer was created
    /// or since the last reset.
    pub fn get_frame_count(&self) -> u64 {
        self.frame_count
    }

    /// Resets the frame counter to zero.
    ///
    /// This method can be useful for benchmarking or when
    /// starting a new rendering session.
    pub fn reset_frame_count(&mut self) {
        self.frame_count = 0;
    }
}

impl<NumberType> Default for CpuReferenceRenderer<NumberType>
where
    NumberType: Copy + Default + PartialEq + Add<Output = NumberType> + AddAssign + Send + Sync,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<NumberType> Renderer for CpuReferenceRenderer<NumberType>
where
    NumberType: Copy + Default + PartialEq + Add<Output = NumberType> + AddAssign + Send + Sync,
{
    fn start(&mut self) -> Result<(), String> {
        if self.is_running {
            return Err("CpuReferenceRenderer is already running".into());
        }

        self.is_running = true;
        println!("CpuReferenceRenderer started");
        Ok(())
    }

    fn stop(&mut self) {
        self.is_running = false;
        println!("CpuReferenceRenderer stopped");
    }

    fn render_frame(&mut self) -> Result<(), String> {
        if !self.is_running {
            return Err("CpuReferenceRenderer is not running".into());
        }

        // Increment frame counter
        self.frame_count += 1;

        println!("CpuReferenceRenderer rendering frame #{}", self.frame_count);
        Ok(())
    }

    fn name(&self) -> &'static str {
        "cpu_reference"
    }
}

// Type aliases for commonly used instantiations
/// Single-precision CPU reference renderer
pub type CpuReferenceRendererFloat32 = CpuReferenceRenderer<f32>;

/// Double-precision CPU reference renderer
pub type CpuReferenceRendererFloat64 = CpuReferenceRenderer<f64>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpu_reference_renderer_new() {
        let renderer: CpuReferenceRenderer<f32> = CpuReferenceRenderer::new();
        assert_eq!(renderer.is_running, false);
        assert_eq!(renderer.frame_count, 0);
    }

    #[test]
    fn test_cpu_reference_renderer_f64_new() {
        let renderer: CpuReferenceRenderer<f64> = CpuReferenceRenderer::new();
        assert_eq!(renderer.is_running, false);
        assert_eq!(renderer.frame_count, 0);
    }

    #[test]
    fn test_renderer_lifecycle() {
        let mut renderer: CpuReferenceRenderer<f32> = CpuReferenceRenderer::new();

        // Test initial state
        assert_eq!(renderer.is_running, false);
        assert_eq!(renderer.frame_count, 0);

        // Test starting the renderer
        assert!(renderer.start().is_ok());
        assert_eq!(renderer.is_running, true);

        // Test rendering frames
        assert!(renderer.render_frame().is_ok());
        assert_eq!(renderer.frame_count, 1);

        assert!(renderer.render_frame().is_ok());
        assert_eq!(renderer.frame_count, 2);

        // Test stopping the renderer
        renderer.stop();
        assert_eq!(renderer.is_running, false);
        assert_eq!(renderer.frame_count, 2); // Frame count should persist
    }

    #[test]
    fn test_frame_count_operations() {
        let mut renderer: CpuReferenceRenderer<f32> = CpuReferenceRenderer::new();
        renderer.start().unwrap();

        // Render some frames
        for _ in 0..5 {
            renderer.render_frame().unwrap();
        }

        assert_eq!(renderer.get_frame_count(), 5);

        // Reset frame count
        renderer.reset_frame_count();
        assert_eq!(renderer.get_frame_count(), 0);
    }

    #[test]
    fn test_cannot_render_when_not_running() {
        let mut renderer: CpuReferenceRenderer<f32> = CpuReferenceRenderer::new();
        let result = renderer.render_frame();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not running"));
    }

    #[test]
    fn test_cannot_start_when_already_running() {
        let mut renderer: CpuReferenceRenderer<f32> = CpuReferenceRenderer::new();
        renderer.start().unwrap();

        let result = renderer.start();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("already running"));
    }

    #[test]
    fn test_type_aliases() {
        let _renderer_f32: CpuReferenceRendererFloat32 = CpuReferenceRenderer::new();
        let _renderer_f64: CpuReferenceRendererFloat64 = CpuReferenceRenderer::new();
    }
}