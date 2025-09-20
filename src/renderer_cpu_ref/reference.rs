use std::path::Path;
use super::scene::Scene;

/// Reference CPU renderer for Fuller.
///
/// Generic over scene precision `T`.
pub fn render_frame_to_png<T>(_scene: &Scene<T>, path: &Path) {
    let width = 2;
    let height = 2;

    // Placeholder: Scene could influence pixels
    // For now, we just pick fixed test colors.
    let buffer: [u8; 16] = [
        255, 0, 0, 255,     // Red
        0, 255, 0, 255,     // Green
        0, 0, 255, 255,     // Blue
        255, 255, 0, 255,   // Yellow
    ];

    image::save_buffer(
        path,
        &buffer,
        width,
        height,
        image::ColorType::Rgba8,
    ).expect("Failed to save test PNG");
}
