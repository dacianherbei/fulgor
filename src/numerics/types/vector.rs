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

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Vector2<T: FloatingPoint = f32> {
    pub x: T,
    pub y: T,
}

// Conditional impls for serde
impl<T> Serialize for Vector2<T>
where
    T: FloatingPoint + Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        (&self.x, &self.y).serialize(serializer)
    }
}

impl<'de, T> Deserialize<'de> for Vector2<T>
where
    T: FloatingPoint + Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let (x, y) = <(T, T)>::deserialize(deserializer)?;
        Ok(Vector2 { x, y })
    }
}

impl<T: FloatingPoint> Vector2<T> {
    /// Construct a new Vector3
    pub fn new(x: T, y: T) -> Self {
        Self { x, y }
    }

    /// Zero vector constructor (requires that 0.0 can be constructed from
    /// the numeric type — for minimalism we do not implement a generic zero;
    /// users may construct via Vector2::new).
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
        }
    }
}

/// Convenience alias so code can refer to Vector3<T> if desired.
pub type Vector2Float<T = f32> = Vector2<T>;

// Implement operator + for Vector2<T>
impl<T: FloatingPoint> Add for Vector2<T> {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self::new(self.x + other.x, self.y + other.y)
    }
}

// Implement operator - for Vector2<T>
impl<T: FloatingPoint> Sub for Vector2<T> {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Self::new(self.x - other.x, self.y - other.y)
    }
}

// Conversions between Vector2<T> and tuples

impl<T: FloatingPoint> From<(T, T)> for Vector2<T> {
    fn from(tuple: (T, T)) -> Self {
        Self {
            x: tuple.0,
            y: tuple.1,
        }
    }
}

impl<T: FloatingPoint> Into<(T, T)> for Vector2<T> {
    fn into(self) -> (T, T) {
        (self.x, self.y)
    }
}

// Conversions between Vector2<T> and arrays [T; 2]

impl<T: FloatingPoint> From<[T; 2]> for Vector2<T> {
    fn from(array: [T; 2]) -> Self {
        Self {
            x: array[0],
            y: array[1],
        }
    }
}

impl<T: FloatingPoint> Into<[T; 2]> for Vector2<T> {
    fn into(self) -> [T; 2] {
        [self.x, self.y]
    }
}

// Conversions from references to Vector2<T>

impl<T: FloatingPoint> From<&(T, T)> for Vector2<T> {
    fn from(tuple: &(T, T)) -> Self {
        Self {
            x: tuple.0,
            y: tuple.1,
        }
    }
}

impl<T: FloatingPoint> From<&[T; 2]> for Vector2<T> {
    fn from(array: &[T; 2]) -> Self {
        Self {
            x: array[0],
            y: array[1],
        }
    }
}

// Reverse conversions: from &Vector2<T> into tuples and arrays

impl<T: FloatingPoint> From<&Vector2<T>> for (T, T) {
    fn from(v: &Vector2<T>) -> Self {
        (v.x, v.y)
    }
}

impl<T: FloatingPoint> From<&Vector2<T>> for [T; 2] {
    fn from(v: &Vector2<T>) -> Self {
        [v.x, v.y]
    }
}

// Implement some f32-specific utilities (square root usage allowed only for f32)
impl Vector2<f32> {
    /// Return the squared length (avoids sqrt)
    pub fn length_squared(&self) -> f32 {
        self.x * self.x + self.y * self.y
    }

    /// Return the Euclidean length. Uses `f32::sqrt`.
    pub fn length(&self) -> f32 {
        self.length_squared().sqrt()
    }

    /// Dot product for f32 specialization
    pub fn dot(&self, other: &Self) -> f32 {
        self.x * other.x + self.y * other.y
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Vector4<T: FloatingPoint = f32> {
    pub x: T,
    pub y: T,
    pub z: T,
    pub w: T,
}

// Conditional impls for serde
impl<T> Serialize for Vector4<T>
where
    T: FloatingPoint + Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        (&self.x, &self.y, &self.z, &self.w).serialize(serializer)
    }
}

impl<'de, T> Deserialize<'de> for Vector4<T>
where
    T: FloatingPoint + Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let (x, y, z, w) = <(T, T, T, T)>::deserialize(deserializer)?;
        Ok(Vector4 { x, y, z, w })
    }
}

impl<T: FloatingPoint> Vector4<T> {
    /// Construct a new Vector4
    pub fn new(x: T, y: T, z: T, w: T) -> Self {
        Self { x, y, z, w }
    }

    /// Zero vector constructor (requires that 0.0 can be constructed from
    /// the numeric type — for minimalism we do not implement a generic zero;
    /// users may construct via Vector4::new).
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
            w: T::zero(),
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
            w: T::one(),
        }
    }
}

/// Convenience alias so code can refer to Vector4<T> if desired.
pub type Vector4Float<T = f32> = Vector4<T>;

// Implement operator + for Vector4<T>
impl<T: FloatingPoint> Add for Vector4<T> {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self::new(self.x + other.x, self.y + other.y,self.z + other.z, self.w + other.w)
    }
}

// Implement operator - for Vector4<T>
impl<T: FloatingPoint> Sub for Vector4<T> {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Self::new(self.x - other.x, self.y - other.y, self.z - other.z, self.w - other.w)
    }
}

// Conversions between Vector4<T> and tuples

impl<T: FloatingPoint> From<(T, T, T, T)> for Vector4<T> {
    fn from(tuple: (T, T, T, T)) -> Self {
        Self {
            x: tuple.0,
            y: tuple.1,
            z: tuple.2,
            w: tuple.3,
        }
    }
}

impl<T: FloatingPoint> Into<(T, T, T, T)> for Vector4<T> {
    fn into(self) -> (T, T, T, T) {
        (self.x, self.y, self.z, self.w)
    }
}

// Conversions between Vector4<T> and arrays [T; 4]

impl<T: FloatingPoint> From<[T; 4]> for Vector4<T> {
    fn from(array: [T; 4]) -> Self {
        Self {
            x: array[0],
            y: array[1],
            z: array[2],
            w: array[3],
        }
    }
}

impl<T: FloatingPoint> Into<[T; 4]> for Vector4<T> {
    fn into(self) -> [T; 4] {
        [self.x, self.y, self.z, self.w]
    }
}

// Conversions from references to Vector4<T>

impl<T: FloatingPoint> From<&(T, T, T, T)> for Vector4<T> {
    fn from(tuple: &(T, T, T, T)) -> Self {
        Self {
            x: tuple.0,
            y: tuple.1,
            z: tuple.2,
            w: tuple.3,
        }
    }
}

impl<T: FloatingPoint> From<&[T; 4]> for Vector4<T> {
    fn from(array: &[T; 4]) -> Self {
        Self {
            x: array[0],
            y: array[1],
            z: array[2],
            w: array[3],
        }
    }
}

// Reverse conversions: from &Vector2<T> into tuples and arrays

impl<T: FloatingPoint> From<&Vector4<T>> for (T, T, T, T) {
    fn from(v: &Vector4<T>) -> Self {
        (v.x, v.y, v.z, v.w)
    }
}

impl<T: FloatingPoint> From<&Vector4<T>> for [T; 4] {
    fn from(v: &Vector4<T>) -> Self {
        [v.x, v.y, v.z, v.w]
    }
}

// Implement some f32-specific utilities (square root usage allowed only for f32)
impl Vector4<f32> {
    /// Return the squared length (avoids sqrt)
    pub fn length_squared(&self) -> f32 {
        self.x * self.x + self.y * self.y + self.z * self.z + self.w * self.w
    }

    /// Return the Euclidean length. Uses `f32::sqrt`.
    pub fn length(&self) -> f32 {
        self.length_squared().sqrt()
    }

    /// Dot product for f32 specialization
    pub fn dot(&self, other: &Self) -> f32 {
        self.x * other.x + self.y * other.y + self.z * other.z + self.w * other.w
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

    #[test]
    fn test_vector_add_sub_and_print_module_vector2() {
        let a = Vector2::new(1.0_f32, 2.0_f32);
        let b = Vector2::new(4.0_f32, 5.0_f32);

        let sum = a + b;
        assert_eq!(sum, Vector2::new(5.0, 7.0));

        let diff = sum - a;
        assert_eq!(diff, b);

        // Test f32-specific functions
        let lsq = a.length_squared();
        assert!((lsq - 5.0).abs() < 1e-6);  // Changed from 14.0 to 5.0

        let len = a.length();
        assert!((len - (5.0_f32.sqrt())).abs() < 1e-6);  // Changed from 14.0 to 5.0
    }

    #[test]
    fn test_vector_alias_and_generic_type_vector2() {
        // Using the alias Vector2 (defaulted to f32)
        let v_alias: Vector2 = Vector2::new(0.0, 1.0);
        assert_eq!(v_alias.y, 1.0_f32);

        // Using a f64 instantiation
        let v64: Vector2<f64> = Vector2::new(1.0_f64, 2.0_f64);
        let w64: Vector2<f64> = Vector2::new(3.0_f64, 2.0_f64);
        let s64 = v64 + w64;
        assert_eq!(s64, Vector2::new(4.0, 4.0));
    }

    #[test]
    fn test_tuple_conversions_vector2() {
        let tup = (1.0f32, 2.0f32);

        let v: Vector2<f32> = tup.into();
        assert_eq!(v, Vector2::new(1.0, 2.0));

        let back: (f32, f32) = v.into();
        assert_eq!(back, tup);
    }

    #[test]
    fn test_array_conversions_vector2() {
        let arr = [1.0f32, 2.0f32];

        let v: Vector2<f32> = arr.into();
        assert_eq!(v, Vector2::new(1.0, 2.0));

        let back: [f32; 2] = v.into();
        assert_eq!(back, arr);
    }

    #[test]
    fn test_reference_tuple_conversion_vector2() {
        let tup = (1.0f32, 2.0f32);
        let v = Vector2::from(&tup);
        assert_eq!(v, Vector2::new(1.0, 2.0));
    }

    #[test]
    fn test_reference_array_conversion_vector2() {
        let arr = [1.0f32, 2.0f32];
        let v = Vector2::from(&arr);
        assert_eq!(v, Vector2::new(1.0, 2.0));
    }

    #[test]
    fn test_vector_ref_into_tuple_and_array_vector2() {
        let v = Vector2::new(7.0f32, 8.0f32);

        let tup: (f32, f32) = (&v).into();
        assert_eq!(tup, (7.0, 8.0));

        let arr: [f32; 2] = (&v).into();
        assert_eq!(arr, [7.0, 8.0]);
    }

    #[test]
    fn test_bincode_roundtrip_vector2() {
        use bincode;
        let v = Vector2::new(1.0f32, 2.0f32);

        // Serialize to bytes
        let encoded: Vec<u8> = bincode::serialize(&v).expect("serialize failed");
        assert!(!encoded.is_empty());

        // Deserialize back
        let decoded: Vector2<f32> = bincode::deserialize(&encoded).expect("deserialize failed");
        assert_eq!(v, decoded);
    }

    #[test]
    fn test_bincode_generic_roundtrip_vector2() {
        use bincode;

        // f32 works
        let v_f32 = Vector2::new(1.0f32, 2.0f32);
        let enc_f32 = bincode::serialize(&v_f32).unwrap();
        let dec_f32: Vector2<f32> = bincode::deserialize(&enc_f32).unwrap();
        assert_eq!(v_f32, dec_f32);

        // f64 works
        let v_f64 = Vector2::new(10.0f64, 20.0f64);
        let enc_f64 = bincode::serialize(&v_f64).unwrap();
        let dec_f64: Vector2<f64> = bincode::deserialize(&enc_f64).unwrap();
        assert_eq!(v_f64, dec_f64);
    }

    #[test]
    fn test_vector_zero_one_vector2() {
        let z = Vector2::<f32>::zero();
        assert_eq!(z, Vector2::new(0.0, 0.0));

        let o = Vector2::<f32>::one();
        assert_eq!(o, Vector2::new(1.0, 1.0));
    }

    #[test]
    fn test_vector_add_sub_and_print_module_vector4() {
        let a = Vector4::new(1.0_f32, 2.0_f32, 3.0_f32, 4.0_f32);
        let b = Vector4::new(4.0_f32, 5.0_f32, 6.0_f32, 7.0_f32);

        let sum = a + b;
        assert_eq!(sum, Vector4::new(5.0, 7.0, 9.0, 11.0));

        let diff = sum - a;
        assert_eq!(diff, b);

        // Test f32-specific functions
        let lsq = a.length_squared();
        assert!((lsq - 30.0).abs() < 1e-6); // 1² + 2² + 3² + 4² = 30

        let len = a.length();
        assert!((len - (30.0_f32.sqrt())).abs() < 1e-6);
    }

    #[test]
    fn test_vector_alias_and_generic_type_vector4() {
        // Using the alias Vector4 (defaulted to f32)
        let v_alias: Vector4 = Vector4::new(0.0, 0.0, 0.0, 1.0);
        assert_eq!(v_alias.w, 1.0_f32);

        // Using a f64 instantiation
        let v64: Vector4<f64> = Vector4::new(1.0_f64, 2.0_f64, 3.0_f64, 4.0_f64);
        let w64: Vector4<f64> = Vector4::new(4.0_f64, 3.0_f64, 2.0_f64, 1.0_f64);
        let s64 = v64 + w64;
        assert_eq!(s64, Vector4::new(5.0, 5.0, 5.0, 5.0));
    }

    #[test]
    fn test_tuple_conversions_vector4() {
        let tup = (1.0f32, 2.0f32, 3.0f32, 4.0f32);

        let v: Vector4<f32> = tup.into();
        assert_eq!(v, Vector4::new(1.0, 2.0, 3.0, 4.0));

        let back: (f32, f32, f32, f32) = v.into();
        assert_eq!(back, tup);
    }

    #[test]
    fn test_array_conversions_vector4() {
        let arr = [1.0f32, 2.0f32, 3.0f32, 4.0f32];

        let v: Vector4<f32> = arr.into();
        assert_eq!(v, Vector4::new(1.0, 2.0, 3.0, 4.0));

        let back: [f32; 4] = v.into();
        assert_eq!(back, arr);
    }

    #[test]
    fn test_reference_tuple_conversion_vector4() {
        let tup = (1.0f32, 2.0f32, 3.0f32, 4.0f32);
        let v = Vector4::from(&tup);
        assert_eq!(v, Vector4::new(1.0, 2.0, 3.0, 4.0));
    }

    #[test]
    fn test_reference_array_conversion_vector4() {
        let arr = [1.0f32, 2.0f32, 3.0f32, 4.0f32];
        let v = Vector4::from(&arr);
        assert_eq!(v, Vector4::new(1.0, 2.0, 3.0, 4.0));
    }

    #[test]
    fn test_vector_ref_into_tuple_and_array_vector4() {
        let v = Vector4::new(7.0f32, 8.0f32, 9.0f32, 10.0f32);

        let tup: (f32, f32, f32, f32) = (&v).into();
        assert_eq!(tup, (7.0, 8.0, 9.0, 10.0));

        let arr: [f32; 4] = (&v).into();
        assert_eq!(arr, [7.0, 8.0, 9.0, 10.0]);
    }

    #[test]
    fn test_bincode_roundtrip_vector4() {
        use bincode;
        let v = Vector4::new(1.0f32, 2.0f32, 3.0f32, 4.0f32);

        // Serialize to bytes
        let encoded: Vec<u8> = bincode::serialize(&v).expect("serialize failed");
        assert!(!encoded.is_empty());

        // Deserialize back
        let decoded: Vector4<f32> = bincode::deserialize(&encoded).expect("deserialize failed");
        assert_eq!(v, decoded);
    }

    #[test]
    fn test_bincode_generic_roundtrip_vector4() {
        use bincode;

        // f32 works
        let v_f32 = Vector4::new(1.0f32, 2.0f32, 3.0f32, 4.0f32);
        let enc_f32 = bincode::serialize(&v_f32).unwrap();
        let dec_f32: Vector4<f32> = bincode::deserialize(&enc_f32).unwrap();
        assert_eq!(v_f32, dec_f32);

        // f64 works
        let v_f64 = Vector4::new(10.0f64, 20.0f64, 30.0f64, 40.0f64);
        let enc_f64 = bincode::serialize(&v_f64).unwrap();
        let dec_f64: Vector4<f64> = bincode::deserialize(&enc_f64).unwrap();
        assert_eq!(v_f64, dec_f64);
    }

    #[test]
    fn test_vector_zero_one_vector4() {
        let z = Vector4::<f32>::zero();
        assert_eq!(z, Vector4::new(0.0, 0.0, 0.0, 0.0));

        let o = Vector4::<f32>::one();
        assert_eq!(o, Vector4::new(1.0, 1.0, 1.0, 1.0));
    }
}
