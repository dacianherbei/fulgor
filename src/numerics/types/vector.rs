// src/numerics/types/vector.rs
// Vector3 generic implementation with default precision f32.
// Uses the FloatingPoint trait from super::traits.

#![allow(dead_code)]

use core::ops::{Add, Sub};

use super::traits::FloatingPoint;

/// Vector3 is a simple 3D vector type with template-able numeric type.
///
/// The public alias `Vector3<T>` is provided below for compatibility with the
/// "Vector3" name while keeping the full-word struct name as requested.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Vector3<T: FloatingPoint = f32> {
    pub x: T,
    pub y: T,
    pub z: T,
}

impl<T: FloatingPoint> Vector3<T> {
    /// Construct a new Vector3
    pub fn new(x: T, y: T, z: T) -> Self {
        Self { x, y, z }
    }

    /// Zero vector constructor (requires that 0.0 can be constructed from
    /// the numeric type — for minimalism we do not implement a generic zero;
    /// users may construct via Vector3::new).
    ///
    /// Kept minimal to avoid extra trait bounds.
    #[allow(dead_code)]
    pub fn zero(x: T, y: T, z: T) -> Self {
        Self { x, y, z }
    }
}

/// Convenience alias so code can refer to Vector3<T> if desired.
pub type Vector3Float<T = f32> = Vector3<T>;

// Implement operator + for Vector3<T>
impl<T: FloatingPoint> Add for Vector3<T> {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self::new(self.x + other.x, self.y + other.y, self.z + other.z)
    }
}

// Implement operator - for Vector3<T>
impl<T: FloatingPoint> Sub for Vector3<T> {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Self::new(self.x - other.x, self.y - other.y, self.z - other.z)
    }
}

// Implement some f32-specific utilities (square root usage allowed only for f32)
impl Vector3<f32> {
    /// Return the squared length (avoids sqrt)
    pub fn length_squared(&self) -> f32 {
        self.x * self.x + self.y * self.y + self.z * self.z
    }

    /// Return the Euclidean length. Uses `f32::sqrt`.
    pub fn length(&self) -> f32 {
        self.length_squared().sqrt()
    }

    /// Dot product for f32 specialization
    pub fn dot(&self, other: &Self) -> f32 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vector_add_sub_and_print_module() {
        let a = Vector3::new(1.0_f32, 2.0_f32, 3.0_f32);
        let b = Vector3::new(4.0_f32, 5.0_f32, 6.0_f32);

        let sum = a + b;
        assert_eq!(sum, Vector3::new(5.0, 7.0, 9.0));

        let diff = sum - a;
        assert_eq!(diff, b);

        // Test f32-specific functions
        let lsq = a.length_squared();
        assert!((lsq - 14.0).abs() < 1e-6);

        let len = a.length();
        assert!((len - (14.0_f32.sqrt())).abs() < 1e-6);

        println!("module: numerics");
    }

    #[test]
    fn test_vector_alias_and_generic_type() {
        // Using the alias Vector3 (defaulted to f32)
        let v_alias: Vector3 = Vector3::new(0.0, 0.0, 1.0);
        assert_eq!(v_alias.z, 1.0_f32);

        // Using a f64 instantiation
        let v64: Vector3<f64> = Vector3::new(1.0_f64, 2.0_f64, 3.0_f64);
        let w64: Vector3<f64> = Vector3::new(3.0_f64, 2.0_f64, 1.0_f64);
        let s64 = v64 + w64;
        assert_eq!(s64, Vector3::new(4.0, 4.0, 4.0));
    }
}
