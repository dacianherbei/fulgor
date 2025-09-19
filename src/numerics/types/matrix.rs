// src/numerics/types/matrix.rs
// Minimal placeholder for the matrix module referenced by the layout.
// Kept intentionally tiny — expand later as needed.

#![allow(dead_code)]

use serde::{Serialize, Deserialize};

/// Minimal Matrix3x3 type placeholder for now.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Matrix3x3<T: FloatingPoint = f32> {
    pub data: [[T; 3]; 3],
}

impl<T: FloatingPoint> Matrix3x3<T> {
    pub fn new(data: [[T; 3]; 3]) -> Self {
        Self { data }
    }

    /// Construct a new matrix from 3 rows
    pub fn from_rows(r0: [T; 3], r1: [T; 3], r2: [T; 3]) -> Self {
        Self { data: [r0, r1, r2] }
    }

    /// Construct a new matrix from 3 columns
    pub fn from_columns(c0: [T; 3], c1: [T; 3], c2: [T; 3]) -> Self {
        Self {
            data: [
                [c0[0], c1[0], c2[0]],
                [c0[1], c1[1], c2[1]],
                [c0[2], c1[2], c2[2]],
            ],
        }
    }

    /// Get a row by index
    pub fn row(&self, idx: usize) -> [T; 3] {
        self.data[idx]
    }

    /// Get a column by index
    pub fn column(&self, idx: usize) -> [T; 3] {
        [self.data[0][idx], self.data[1][idx], self.data[2][idx]]
    }

    /// Zero matrix
    pub fn zero() -> Self
    where
        T: From<f32>,
    {
        Self {
            data: core::array::from_fn(|_| core::array::from_fn(|_| T::zero())),
        }
    }

    /// One matrix (all elements = 1)
    pub fn one() -> Self
    where
        T: From<f32>,
    {
        Self {
            data: core::array::from_fn(|_| core::array::from_fn(|_| T::one())),
        }
    }

    /// Identity matrix
    pub fn identity() -> Self
    where
        T: From<f32>,
    {
        let mut m = core::array::from_fn(|_| core::array::from_fn(|_| T::zero()));
        for i in 0..3 {
            m[i][i] = T::one();
        }
        Self { data: m }
    }

    /// Swap two rows in place
    pub fn swap_rows(&mut self, r1: usize, r2: usize) {
        self.data.swap(r1, r2);
    }

    /// Multiply a row by a scalar
    pub fn scale_row(&mut self, row: usize, scalar: T) {
        for j in 0..3 {
            self.data[row][j] = self.data[row][j] * scalar;
        }
    }

    /// Add a multiple of one row to another row
    pub fn add_row_multiple(&mut self, target: usize, source: usize, scalar: T) {
        for j in 0..3 {
            self.data[target][j] = self.data[target][j] + self.data[source][j] * scalar;
        }
    }
}

// Generic serde implementations for Matrix3x3
impl<T> Serialize for Matrix3x3<T>
where
    T: FloatingPoint + Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.data.serialize(serializer)
    }
}

impl<'de, T> Deserialize<'de> for Matrix3x3<T>
where
    T: FloatingPoint + Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let arr = <[[T; 3]; 3]>::deserialize(deserializer)?;
        Ok(Matrix3x3 { data: arr })
    }
}

use core::ops::{Add, Sub, Mul};
use crate::numerics::types::traits::FloatingPoint;
use crate::numerics::types::vector::Vector3;

impl<T: FloatingPoint> Add for Matrix3x3<T> {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        let mut result = self.data;
        for i in 0..3 {
            for j in 0..3 {
                result[i][j] = self.data[i][j] + other.data[i][j];
            }
        }
        Matrix3x3 { data: result }
    }
}

impl<T: FloatingPoint> Sub for Matrix3x3<T> {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        let mut result = self.data;
        for i in 0..3 {
            for j in 0..3 {
                result[i][j] = self.data[i][j] - other.data[i][j];
            }
        }
        Matrix3x3 { data: result }
    }
}

impl<T: FloatingPoint> Mul<T> for Matrix3x3<T> {
    type Output = Self;

    fn mul(self, scalar: T) -> Self {
        let mut result = self.data;
        for i in 0..3 {
            for j in 0..3 {
                result[i][j] = self.data[i][j] * scalar;
            }
        }
        Matrix3x3 { data: result }
    }
}

impl<T: FloatingPoint> Mul<Vector3<T>> for Matrix3x3<T> {
    type Output = Vector3<T>;

    fn mul(self, rhs: Vector3<T>) -> Vector3<T> {
        Vector3 {
            x: self.data[0][0] * rhs.x + self.data[0][1] * rhs.y + self.data[0][2] * rhs.z,
            y: self.data[1][0] * rhs.x + self.data[1][1] * rhs.y + self.data[1][2] * rhs.z,
            z: self.data[2][0] * rhs.x + self.data[2][1] * rhs.y + self.data[2][2] * rhs.z,
        }
    }
}

impl<T: FloatingPoint> Mul<Matrix3x3<T>> for Vector3<T> {
    type Output = Vector3<T>;

    fn mul(self, rhs: Matrix3x3<T>) -> Vector3<T> {
        Vector3 {
            x: self.x * rhs.data[0][0] + self.y * rhs.data[1][0] + self.z * rhs.data[2][0],
            y: self.x * rhs.data[0][1] + self.y * rhs.data[1][1] + self.z * rhs.data[2][1],
            z: self.x * rhs.data[0][2] + self.y * rhs.data[1][2] + self.z * rhs.data[2][2],
        }
    }
}

impl<T: FloatingPoint + From<f32>> Mul<Matrix3x3<T>> for Matrix3x3<T> {
    type Output = Matrix3x3<T>;

    fn mul(self, rhs: Matrix3x3<T>) -> Matrix3x3<T> {
        let mut result = [[T::zero(); 3]; 3];
        for i in 0..3 {
            for j in 0..3 {
                result[i][j] =
                    self.data[i][0] * rhs.data[0][j] +
                        self.data[i][1] * rhs.data[1][j] +
                        self.data[i][2] * rhs.data[2][j];
            }
        }
        Matrix3x3 { data: result }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bincode;

    #[test]
    fn test_matrix_roundtrip() {
        let m = Matrix3x3::new([
            [1.0f32, 2.0f32, 3.0f32],
            [4.0f32, 5.0f32, 6.0f32],
            [7.0f32, 8.0f32, 9.0f32],
        ]);

        let encoded = bincode::serialize(&m).unwrap();
        let decoded: Matrix3x3<f32> = bincode::deserialize(&encoded).unwrap();

        assert_eq!(m, decoded);
    }

    #[test]
    fn test_matrix_constructors_and_accessors() {
        let m = Matrix3x3::from_rows(
            [1.0f32, 2.0, 3.0],
            [4.0, 5.0, 6.0],
            [7.0, 8.0, 9.0],
        );

        assert_eq!(m.row(0), [1.0, 2.0, 3.0]);
        assert_eq!(m.column(1), [2.0, 5.0, 8.0]);

        let z = Matrix3x3::<f32>::zero();
        assert_eq!(z, Matrix3x3::new([[0.0; 3]; 3]));

        let o = Matrix3x3::<f32>::one();
        assert_eq!(o, Matrix3x3::new([[1.0; 3]; 3]));

        let id = Matrix3x3::<f32>::identity();
        assert_eq!(id, Matrix3x3::new([[1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0]]));
    }

    #[test]
    fn test_matrix_zero_one_identity() {
        let z = Matrix3x3::<f32>::zero();
        assert_eq!(z, Matrix3x3::new([[0.0; 3]; 3]));

        let o = Matrix3x3::<f32>::one();
        assert_eq!(o, Matrix3x3::new([[1.0; 3]; 3]));

        let id = Matrix3x3::<f32>::identity();
        assert_eq!(
            id,
            Matrix3x3::new([[1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [0.0, 0.0, 1.0]])
        );
    }

    #[test]
    fn test_matrix_add_sub_mul() {
        let a = Matrix3x3::from_rows([1.0, 2.0, 3.0],
                                   [4.0, 5.0, 6.0],
                                   [7.0, 8.0, 9.0]);
        let b = Matrix3x3::from_rows([9.0, 8.0, 7.0],
                                   [6.0, 5.0, 4.0],
                                   [3.0, 2.0, 1.0]);

        let sum = a + b;
        assert_eq!(sum.row(0), [10.0, 10.0, 10.0]);

        let diff = a - b;
        assert_eq!(diff.row(2), [4.0, 6.0, 8.0]);

        let scaled = a * 2.0;
        assert_eq!(scaled.row(1), [8.0, 10.0, 12.0]);
    }

    #[test]
    fn test_matrix_vector_mul() {
        let m = Matrix3x3::from_rows(
            [1.0f32, 2.0, 3.0],
            [4.0, 5.0, 6.0],
            [7.0, 8.0, 9.0],
        );

        let v = Vector3::new(1.0f32, 1.0f32, 1.0f32);
        let result = m * v;

        // Row sums: [6, 15, 24]
        assert_eq!(result, Vector3::new(6.0, 15.0, 24.0));
    }

    #[test]
    fn test_vector_matrix_mul() {
        let m = Matrix3x3::from_rows(
            [1.0f32, 2.0, 3.0],
            [4.0, 5.0, 6.0],
            [7.0, 8.0, 9.0],
        );

        let v = Vector3::new(1.0f32, 1.0f32, 1.0f32);
        let result = v * m;

        // Column sums: [12, 15, 18]
        assert_eq!(result, Vector3::new(12.0, 15.0, 18.0));
    }

    #[test]
    fn test_matrix_matrix_mul() {
        let a = Matrix3x3::from_rows(
            [1.0f32, 2.0, 3.0],
            [4.0, 5.0, 6.0],
            [7.0, 8.0, 9.0],
        );

        let b = Matrix3x3::from_rows(
            [9.0f32, 8.0, 7.0],
            [6.0, 5.0, 4.0],
            [3.0, 2.0, 1.0],
        );

        let c = a * b;

        assert_eq!(c.row(0), [30.0, 24.0, 18.0]);
        assert_eq!(c.row(1), [84.0, 69.0, 54.0]);
        assert_eq!(c.row(2), [138.0, 114.0, 90.0]);
    }

    #[test]
    fn test_row_operations() {
        let mut m = Matrix3x3::identity();

        // swap rows 0 and 1
        m.swap_rows(0, 1);
        assert_eq!(m.row(0), [0.0, 1.0, 0.0]);
        assert_eq!(m.row(1), [1.0, 0.0, 0.0]);

        // scale row 0 by 2
        m.scale_row(0, 2.0);
        assert_eq!(m.row(0), [0.0, 2.0, 0.0]);

        // add row1 * 3 into row0
        m.add_row_multiple(0, 1, 3.0);
        assert_eq!(m.row(0), [3.0, 2.0, 0.0]);
    }

}