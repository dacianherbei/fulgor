// src/numerics/types/point.rs
// Point3 is an alias for Vector3 as requested.

#![allow(dead_code)]

use super::vector::Vector3;

/// Point3 is an alias to Vector3 to represent points in space.
///
/// The alias keeps generic template parameterization.
pub type Point3<T> = Vector3<T>;
pub type Point3Float<T = f32> = Vector3<T>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_point_alias_behaviour() {
        let p: Point3Float = Point3::new(1.0, 2.0, 3.0);
        assert_eq!(p.x, 1.0_f32);
    }

    #[test]
    fn test_point3_bincode_roundtrip() {
        let p: Point3<f64> = Point3::new(1.1, 2.2, 3.3);

        let encoded = bincode::serialize(&p).unwrap();
        let decoded: Point3<f64> = bincode::deserialize(&encoded).unwrap();

        assert_eq!(p, decoded);
    }
}
