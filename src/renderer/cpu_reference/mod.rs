//! CPU reference renderer

use crate::renderer::Renderer;

pub struct CpuReferenceRenderer {
    running: bool,
}

impl CpuReferenceRenderer {
    pub fn new() -> Self {
        Self { running: false }
    }
}

impl Renderer for CpuReferenceRenderer {
    fn start(&mut self) -> Result<(), String> {
        self.running = true;
        println!("CpuReferenceRenderer started");
        Ok(())
    }

    fn stop(&mut self) {
        self.running = false;
        println!("CpuReferenceRenderer stopped");
    }

    fn render_frame(&mut self) -> Result<(), String> {
        if !self.running {
            return Err("CpuReferenceRenderer not running".into());
        }
        println!("CpuReferenceRenderer rendering a frame");
        Ok(())
    }

    fn name(&self) -> &'static str {
        "cpu_reference"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpu_reference() {
        let mut r = CpuReferenceRenderer::new();
        r.start().unwrap();
        r.render_frame().unwrap();
        r.stop();
    }
}
