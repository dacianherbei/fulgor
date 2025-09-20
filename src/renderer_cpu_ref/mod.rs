pub mod reference;
pub mod scene;

#[cfg(test)]
mod tests {
    use super::reference::render_frame_to_png;
    use super::scene::Scene;
    use std::path::Path;

    #[test]
    fn create_test_frame() {
        let path = Path::new("test_output.png");

        // Create a dummy scene
        let scene = Scene::<f32>::new("Test Scene");

        // Call renderer
        render_frame_to_png(&scene, path);

        println!("Test completed: module = renderer_cpu_ref, output = {:?}", path);
    }
}
