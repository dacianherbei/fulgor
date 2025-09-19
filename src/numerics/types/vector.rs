// src/numerics/types/vector.rs
// Vector3 generic implementation with default precision f32.
// Uses the FloatingPoint trait from super::traits.

#![allow(dead_code)]

use core::ops::{Add, Sub};
use serde::{Serialize, Deserialize};

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

// Conditional impls for serde
impl<T> Serialize for Vector3<T>
where
    T: FloatingPoint + Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        (&self.x, &self.y, &self.z).serialize(serializer)
    }
}

impl<'de, T> Deserialize<'de> for Vector3<T>
where
    T: FloatingPoint + Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let (x, y, z) = <(T, T, T)>::deserialize(deserializer)?;
        Ok(Vector3 { x, y, z })
    }
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
    /// Vector of all zeros
    #[allow(dead_code)]
    pub fn zero() -> Self
    where
        T: From<f32>,
    {
        Self {
            x: T::zero(),
            y: T::zero(),
            z: T::zero(),
        }
    }

    /// Vector of all ones
    pub fn one() -> Self
    where
        T: From<f32>,
    {
        Self {
            x: T::one(),
            y: T::one(),
            z: T::one(),
        }
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

// Conversions between Vector3<T> and tuples

impl<T: FloatingPoint> From<(T, T, T)> for Vector3<T> {
    fn from(tuple: (T, T, T)) -> Self {
        Self {
            x: tuple.0,
            y: tuple.1,
            z: tuple.2,
        }
    }
}

impl<T: FloatingPoint> Into<(T, T, T)> for Vector3<T> {
    fn into(self) -> (T, T, T) {
        (self.x, self.y, self.z)
    }
}

// Conversions between Vector3<T> and arrays [T; 3]

impl<T: FloatingPoint> From<[T; 3]> for Vector3<T> {
    fn from(array: [T; 3]) -> Self {
        Self {
            x: array[0],
            y: array[1],
            z: array[2],
        }
    }
}

impl<T: FloatingPoint> Into<[T; 3]> for Vector3<T> {
    fn into(self) -> [T; 3] {
        [self.x, self.y, self.z]
    }
}

// Conversions from references to Vector3<T>

impl<T: FloatingPoint> From<&(T, T, T)> for Vector3<T> {
    fn from(tuple: &(T, T, T)) -> Self {
        Self {
            x: tuple.0,
            y: tuple.1,
            z: tuple.2,
        }
    }
}

impl<T: FloatingPoint> From<&[T; 3]> for Vector3<T> {
    fn from(array: &[T; 3]) -> Self {
        Self {
            x: array[0],
            y: array[1],
            z: array[2],
        }
    }
}

// Reverse conversions: from &Vector3<T> into tuples and arrays

impl<T: FloatingPoint> From<&Vector3<T>> for (T, T, T) {
    fn from(v: &Vector3<T>) -> Self {
        (v.x, v.y, v.z)
    }
}

impl<T: FloatingPoint> From<&Vector3<T>> for [T; 3] {
    fn from(v: &Vector3<T>) -> Self {
        [v.x, v.y, v.z]
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

    #[test]
    fn test_tuple_conversions() {
        let tup = (1.0f32, 2.0f32, 3.0f32);

        let v: Vector3<f32> = tup.into();
        assert_eq!(v, Vector3::new(1.0, 2.0, 3.0));

        let back: (f32, f32, f32) = v.into();
        assert_eq!(back, tup);
    }

    #[test]
    fn test_array_conversions() {
        let arr = [1.0f32, 2.0f32, 3.0f32];

        let v: Vector3<f32> = arr.into();
        assert_eq!(v, Vector3::new(1.0, 2.0, 3.0));

        let back: [f32; 3] = v.into();
        assert_eq!(back, arr);
    }

    #[test]
    fn test_reference_tuple_conversion() {
        let tup = (1.0f32, 2.0f32, 3.0f32);
        let v = Vector3::from(&tup);
        assert_eq!(v, Vector3::new(1.0, 2.0, 3.0));
    }

    #[test]
    fn test_reference_array_conversion() {
        let arr = [1.0f32, 2.0f32, 3.0f32];
        let v = Vector3::from(&arr);
        assert_eq!(v, Vector3::new(1.0, 2.0, 3.0));
    }

    #[test]
    fn test_vector_ref_into_tuple_and_array() {
        let v = Vector3::new(7.0f32, 8.0f32, 9.0f32);

        let tup: (f32, f32, f32) = (&v).into();
        assert_eq!(tup, (7.0, 8.0, 9.0));

        let arr: [f32; 3] = (&v).into();
        assert_eq!(arr, [7.0, 8.0, 9.0]);
    }

    #[test]
    fn test_bincode_roundtrip() {
        use bincode;
        let v = Vector3::new(1.0f32, 2.0f32, 3.0f32);

        // Serialize to bytes
        let encoded: Vec<u8> = bincode::serialize(&v).expect("serialize failed");
        assert!(!encoded.is_empty());

        // Deserialize back
        let decoded: Vector3<f32> = bincode::deserialize(&encoded).expect("deserialize failed");
        assert_eq!(v, decoded);
    }

    #[test]
    fn test_bincode_generic_roundtrip() {
        use bincode;

        // f32 works
        let v_f32 = Vector3::new(1.0f32, 2.0f32, 3.0f32);
        let enc_f32 = bincode::serialize(&v_f32).unwrap();
        let dec_f32: Vector3<f32> = bincode::deserialize(&enc_f32).unwrap();
        assert_eq!(v_f32, dec_f32);

        // f64 works
        let v_f64 = Vector3::new(10.0f64, 20.0f64, 30.0f64);
        let enc_f64 = bincode::serialize(&v_f64).unwrap();
        let dec_f64: Vector3<f64> = bincode::deserialize(&enc_f64).unwrap();
        assert_eq!(v_f64, dec_f64);
    }

    #[test]
    fn test_vector_zero_one() {
        let z = Vector3::<f32>::zero();
        assert_eq!(z, Vector3::new(0.0, 0.0, 0.0));

        let o = Vector3::<f32>::one();
        assert_eq!(o, Vector3::new(1.0, 1.0, 1.0));
    }

}
