//! Optional GPU renderer (feature-gated)

pub mod backend;

use crate::renderer::Renderer;

pub struct GpuOptionalRenderer {
    running: bool,
}

impl GpuOptionalRenderer {
    pub fn new() -> Self {
        Self { running: false }
    }
}

impl Renderer for GpuOptionalRenderer {
    fn start(&mut self) -> Result<(), String> {
        #[cfg(not(feature = "gpu"))]
        {
            return Err("gpu feature not enabled".into());
        }

        #[cfg(feature = "gpu")]
        {
            backend::initialize_gpu_backend()?;
            self.running = true;
            println!("GpuOptionalRenderer started");
            Ok(())
        }
    }

    fn stop(&mut self) {
        self.running = false;
        println!("GpuOptionalRenderer stopped");
    }

    fn render_frame(&mut self) -> Result<(), String> {
        if !self.running {
            return Err("GpuOptionalRenderer not running".into());
        }
        println!("GpuOptionalRenderer rendering a frame");
        Ok(())
    }

    fn name(&self) -> &'static str {
        "gpu_optional"
    }
}
