//! World and scene management for the fulgor renderer.
//!
//! This module provides abstractions for managing 3D worlds, scenes, and the objects
//! within them. It includes support for 3D Gaussian splats, camera management,
//! and scene graph operations optimized for the tile-based rendering pipeline.

use crate::renderer::DataPrecision;
use std::collections::HashMap;

/// A 3D point with associated data precision.
#[derive(Debug, Clone, PartialEq)]
pub struct Point3D {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Point3D {
    pub fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    pub fn origin() -> Self {
        Self::new(0.0, 0.0, 0.0)
    }

    /// Convert to the specified precision for calculations.
    pub fn to_precision(&self, precision: DataPrecision) -> PrecisionPoint3D {
        match precision {
            DataPrecision::F16 => PrecisionPoint3D::F16([self.x as f32, self.y as f32, self.z as f32]), // Note: Using f32 as proxy for f16
            DataPrecision::F32 => PrecisionPoint3D::F32([self.x as f32, self.y as f32, self.z as f32]),
            DataPrecision::F64 => PrecisionPoint3D::F64([self.x, self.y, self.z]),
            DataPrecision::BFloat16 => PrecisionPoint3D::BFloat16([self.x as f32, self.y as f32, self.z as f32]),
        }
    }
}

/// A 3D point with precision-specific storage.
#[derive(Debug, Clone, PartialEq)]
pub enum PrecisionPoint3D {
    F16([f32; 3]),      // Using f32 as proxy for f16
    F32([f32; 3]),
    F64([f64; 3]),
    BFloat16([f32; 3]), // Using f32 as proxy for bfloat16
}

impl PrecisionPoint3D {
    pub fn precision(&self) -> DataPrecision {
        match self {
            PrecisionPoint3D::F16(_) => DataPrecision::F16,
            PrecisionPoint3D::F32(_) => DataPrecision::F32,
            PrecisionPoint3D::F64(_) => DataPrecision::F64,
            PrecisionPoint3D::BFloat16(_) => DataPrecision::BFloat16,
        }
    }

    pub fn to_f64_array(&self) -> [f64; 3] {
        match self {
            PrecisionPoint3D::F16([x, y, z]) => [*x as f64, *y as f64, *z as f64],
            PrecisionPoint3D::F32([x, y, z]) => [*x as f64, *y as f64, *z as f64],
            PrecisionPoint3D::F64([x, y, z]) => [*x, *y, *z],
            PrecisionPoint3D::BFloat16([x, y, z]) => [*x as f64, *y as f64, *z as f64],
        }
    }
}

/// A 3D Gaussian splat with position, covariance, and appearance data.
#[derive(Debug, Clone)]
pub struct GaussianSplat {
    /// Unique identifier for this splat
    pub id: u64,

    /// 3D position of the Gaussian center
    pub position: Point3D,

    /// Covariance matrix (stored as upper triangular: xx, xy, xz, yy, yz, zz)
    pub covariance: [f64; 6],

    /// Color as RGBA values (0.0 to 1.0)
    pub color: [f64; 4],

    /// Opacity/alpha value (0.0 to 1.0)
    pub opacity: f64,

    /// Optional metadata for this splat
    pub metadata: HashMap<String, String>,
}

impl GaussianSplat {
    /// Create a new Gaussian splat with default values.
    pub fn new(id: u64, position: Point3D) -> Self {
        Self {
            id,
            position,
            covariance: [1.0, 0.0, 0.0, 1.0, 0.0, 1.0], // Identity-like covariance
            color: [1.0, 1.0, 1.0, 1.0], // White
            opacity: 1.0,
            metadata: HashMap::new(),
        }
    }

    /// Set the color of this splat.
    pub fn with_color(mut self, r: f64, g: f64, b: f64, a: f64) -> Self {
        self.color = [r, g, b, a];
        self
    }

    /// Set the opacity of this splat.
    pub fn with_opacity(mut self, opacity: f64) -> Self {
        self.opacity = opacity.clamp(0.0, 1.0);
        self
    }

    /// Set the covariance matrix for this splat.
    pub fn with_covariance(mut self, covariance: [f64; 6]) -> Self {
        self.covariance = covariance;
        self
    }

    /// Add metadata to this splat.
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }
}

/// Camera configuration for rendering.
#[derive(Debug, Clone)]
pub struct Camera {
    /// Camera position in world space
    pub position: Point3D,

    /// Camera target/look-at point
    pub target: Point3D,

    /// Up vector for camera orientation
    pub up: Point3D,

    /// Field of view in degrees
    pub fov: f64,

    /// Aspect ratio (width/height)
    pub aspect_ratio: f64,

    /// Near clipping plane distance
    pub near: f64,

    /// Far clipping plane distance
    pub far: f64,
}

impl Camera {
    /// Create a new camera with default settings.
    pub fn new() -> Self {
        Self {
            position: Point3D::new(0.0, 0.0, 5.0),
            target: Point3D::origin(),
            up: Point3D::new(0.0, 1.0, 0.0),
            fov: 45.0,
            aspect_ratio: 16.0 / 9.0,
            near: 0.1,
            far: 1000.0,
        }
    }

    /// Set the camera position.
    pub fn with_position(mut self, position: Point3D) -> Self {
        self.position = position;
        self
    }

    /// Set the camera target.
    pub fn with_target(mut self, target: Point3D) -> Self {
        self.target = target;
        self
    }

    /// Set the field of view.
    pub fn with_fov(mut self, fov: f64) -> Self {
        self.fov = fov.clamp(1.0, 179.0);
        self
    }

    /// Set the aspect ratio.
    pub fn with_aspect_ratio(mut self, aspect_ratio: f64) -> Self {
        self.aspect_ratio = aspect_ratio.max(0.1);
        self
    }
}

impl Default for Camera {
    fn default() -> Self {
        Self::new()
    }
}

/// A 3D world containing Gaussian splats and rendering configuration.
#[derive(Debug, Clone)]
pub struct World {
    /// Collection of Gaussian splats in this world
    pub splats: Vec<GaussianSplat>,

    /// Camera configuration for rendering this world
    pub camera: Camera,

    /// Background color as RGBA
    pub background_color: [f64; 4],

    /// World-specific metadata
    pub metadata: HashMap<String, String>,

    /// Next available splat ID
    next_id: u64,
}

impl World {
    /// Create a new empty world.
    pub fn new() -> Self {
        Self {
            splats: Vec::new(),
            camera: Camera::default(),
            background_color: [0.0, 0.0, 0.0, 1.0], // Black background
            metadata: HashMap::new(),
            next_id: 0,
        }
    }

    /// Add a Gaussian splat to the world.
    pub fn add_splat(&mut self, mut splat: GaussianSplat) -> u64 {
        if splat.id == 0 {
            splat.id = self.next_id();
        }
        let id = splat.id;
        self.splats.push(splat);
        id
    }

    /// Create and add a new splat at the specified position.
    pub fn add_splat_at(&mut self, position: Point3D) -> u64 {
        let id = self.next_id();
        let splat = GaussianSplat::new(id, position);
        self.splats.push(splat);
        id
    }

    /// Remove a splat by its ID.
    pub fn remove_splat(&mut self, id: u64) -> Option<GaussianSplat> {
        if let Some(pos) = self.splats.iter().position(|s| s.id == id) {
            Some(self.splats.remove(pos))
        } else {
            None
        }
    }

    /// Get a splat by its ID.
    pub fn get_splat(&self, id: u64) -> Option<&GaussianSplat> {
        self.splats.iter().find(|s| s.id == id)
    }

    /// Get a mutable reference to a splat by its ID.
    pub fn get_splat_mut(&mut self, id: u64) -> Option<&mut GaussianSplat> {
        self.splats.iter_mut().find(|s| s.id == id)
    }

    /// Get the total number of splats in this world.
    pub fn splat_count(&self) -> usize {
        self.splats.len()
    }

    /// Clear all splats from the world.
    pub fn clear_splats(&mut self) {
        self.splats.clear();
    }

    /// Set the camera for this world.
    pub fn set_camera(&mut self, camera: Camera) {
        self.camera = camera;
    }

    /// Set the background color.
    pub fn set_background_color(&mut self, r: f64, g: f64, b: f64, a: f64) {
        self.background_color = [r, g, b, a];
    }

    /// Add metadata to the world.
    pub fn set_metadata(&mut self, key: String, value: String) {
        self.metadata.insert(key, value);
    }

    /// Get metadata from the world.
    pub fn get_metadata(&self, key: &str) -> Option<&String> {
        self.metadata.get(key)
    }

    /// Generate the next available splat ID.
    fn next_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    /// Calculate the bounding box of all splats in the world.
    pub fn bounding_box(&self) -> Option<(Point3D, Point3D)> {
        if self.splats.is_empty() {
            return None;
        }

        let mut min = self.splats[0].position.clone();
        let mut max = self.splats[0].position.clone();

        for splat in &self.splats {
            min.x = min.x.min(splat.position.x);
            min.y = min.y.min(splat.position.y);
            min.z = min.z.min(splat.position.z);

            max.x = max.x.max(splat.position.x);
            max.y = max.y.max(splat.position.y);
            max.z = max.z.max(splat.position.z);
        }

        Some((min, max))
    }
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_point3d_creation() {
        let point = Point3D::new(1.0, 2.0, 3.0);
        assert_eq!(point.x, 1.0);
        assert_eq!(point.y, 2.0);
        assert_eq!(point.z, 3.0);

        let origin = Point3D::origin();
        assert_eq!(origin.x, 0.0);
        assert_eq!(origin.y, 0.0);
        assert_eq!(origin.z, 0.0);
    }

    #[test]
    fn test_point3d_precision_conversion() {
        let point = Point3D::new(1.5, 2.5, 3.5);

        let f32_point = point.to_precision(DataPrecision::F32);
        assert_eq!(f32_point.precision(), DataPrecision::F32);

        let f64_point = point.to_precision(DataPrecision::F64);
        assert_eq!(f64_point.precision(), DataPrecision::F64);

        match f64_point {
            PrecisionPoint3D::F64([x, y, z]) => {
                assert_eq!(x, 1.5);
                assert_eq!(y, 2.5);
                assert_eq!(z, 3.5);
            }
            _ => panic!("Expected F64 precision point"),
        }
    }

    #[test]
    fn test_gaussian_splat_creation() {
        let position = Point3D::new(1.0, 2.0, 3.0);
        let splat = GaussianSplat::new(42, position.clone())
            .with_color(0.5, 0.6, 0.7, 0.8)
            .with_opacity(0.9)
            .with_metadata("test".to_string(), "value".to_string());

        assert_eq!(splat.id, 42);
        assert_eq!(splat.position, position);
        assert_eq!(splat.color, [0.5, 0.6, 0.7, 0.8]);
        assert_eq!(splat.opacity, 0.9);
        assert_eq!(splat.metadata.get("test"), Some(&"value".to_string()));
    }

    #[test]
    fn test_camera_creation() {
        let camera = Camera::new()
            .with_position(Point3D::new(1.0, 2.0, 3.0))
            .with_target(Point3D::new(4.0, 5.0, 6.0))
            .with_fov(60.0)
            .with_aspect_ratio(1.5);

        assert_eq!(camera.position, Point3D::new(1.0, 2.0, 3.0));
        assert_eq!(camera.target, Point3D::new(4.0, 5.0, 6.0));
        assert_eq!(camera.fov, 60.0);
        assert_eq!(camera.aspect_ratio, 1.5);
    }

    #[test]
    fn test_world_operations() {
        let mut world = World::new();
        assert_eq!(world.splat_count(), 0);

        // Add splats
        let id1 = world.add_splat_at(Point3D::new(1.0, 0.0, 0.0));
        let id2 = world.add_splat_at(Point3D::new(0.0, 1.0, 0.0));
        assert_eq!(world.splat_count(), 2);

        // Get splats
        assert!(world.get_splat(id1).is_some());
        assert!(world.get_splat(id2).is_some());
        assert!(world.get_splat(999).is_none());

        // Remove splat
        let removed = world.remove_splat(id1);
        assert!(removed.is_some());
        assert_eq!(world.splat_count(), 1);
        assert!(world.get_splat(id1).is_none());

        // Clear all
        world.clear_splats();
        assert_eq!(world.splat_count(), 0);
    }

    #[test]
    fn test_world_bounding_box() {
        let mut world = World::new();

        // Empty world has no bounding box
        assert!(world.bounding_box().is_none());

        // Add some splats
        world.add_splat_at(Point3D::new(-1.0, -2.0, -3.0));
        world.add_splat_at(Point3D::new(2.0, 1.0, 3.0));
        world.add_splat_at(Point3D::new(0.0, 0.0, 0.0));

        let (min, max) = world.bounding_box().unwrap();
        assert_eq!(min, Point3D::new(-1.0, -2.0, -3.0));
        assert_eq!(max, Point3D::new(2.0, 1.0, 3.0));
    }

    #[test]
    fn test_world_metadata() {
        let mut world = World::new();

        assert!(world.get_metadata("test").is_none());

        world.set_metadata("test".to_string(), "value".to_string());
        assert_eq!(world.get_metadata("test"), Some(&"value".to_string()));

        world.set_background_color(0.1, 0.2, 0.3, 0.4);
        assert_eq!(world.background_color, [0.1, 0.2, 0.3, 0.4]);
    }
}