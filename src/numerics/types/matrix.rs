// src/numerics/types/matrix.rs
// Minimal placeholder for the matrix module referenced by the layout.
// Kept intentionally tiny — expand later as needed.

#![allow(dead_code)]

use crate::numerics::types::traits::FloatingPoint;
use crate::numerics::types::vector::Vector2;
use crate::numerics::types::vector::Vector3;
use serde::{Deserialize, Serialize};
use std::ops::{Add, Index, IndexMut, Mul, Sub, AddAssign, Div, MulAssign, Neg, SubAssign};
use std::fmt;

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
                result[i][j] = self.data[i][0] * rhs.data[0][j]
                    + self.data[i][1] * rhs.data[1][j]
                    + self.data[i][2] * rhs.data[2][j];
            }
        }
        Matrix3x3 { data: result }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Matrix2x2<T: FloatingPoint = f32> {
    pub data: [[T; 2]; 2],
}

impl<T: FloatingPoint> Matrix2x2<T> {
    /// Constructor
    pub fn new(data: [[T; 2]; 2]) -> Self {
        Self { data }
    }

    pub fn from_rows(r0: [T; 2], r1: [T; 2]) -> Self {
        Self { data: [r0, r1] }
    }

    pub fn from_columns(c0: [T; 2], c1: [T; 2]) -> Self {
        Self {
            data: [[c0[0], c1[0]], [c0[1], c1[1]]],
        }
    }

    /// Constants
    pub fn zero() -> Self {
        Self {
            data: [[T::zero(); 2]; 2],
        }
    }

    pub fn one() -> Self {
        Self {
            data: [[T::one(); 2]; 2],
        }
    }

    pub fn identity() -> Self {
        Self {
            data: [[T::one(), T::zero()], [T::zero(), T::one()]],
        }
    }

    /// Row operations
    pub fn swap_rows(&mut self, r1: usize, r2: usize) {
        self.data.swap(r1, r2);
    }

    pub fn scale_row(&mut self, row: usize, scalar: T) {
        for j in 0..2 {
            self.data[row][j] = self.data[row][j] * scalar;
        }
    }

    pub fn add_row_multiple(&mut self, target: usize, source: usize, scalar: T) {
        for j in 0..2 {
            self.data[target][j] = self.data[target][j] + self.data[source][j] * scalar;
        }
    }

    /// Transpose
    pub fn transpose(&self) -> Self {
        Self {
            data: [
                [self.data[0][0], self.data[1][0]],
                [self.data[0][1], self.data[1][1]],
            ],
        }
    }

    /// Determinant
    pub fn determinant(&self) -> T {
        self.data[0][0] * self.data[1][1] - self.data[0][1] * self.data[1][0]
    }
}

/// Arithmetic
impl<T: FloatingPoint> Add for Matrix2x2<T> {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Self {
            data: [
                [
                    self.data[0][0] + rhs.data[0][0],
                    self.data[0][1] + rhs.data[0][1],
                ],
                [
                    self.data[1][0] + rhs.data[1][0],
                    self.data[1][1] + rhs.data[1][1],
                ],
            ],
        }
    }
}

impl<T: FloatingPoint> Sub for Matrix2x2<T> {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            data: [
                [
                    self.data[0][0] - rhs.data[0][0],
                    self.data[0][1] - rhs.data[0][1],
                ],
                [
                    self.data[1][0] - rhs.data[1][0],
                    self.data[1][1] - rhs.data[1][1],
                ],
            ],
        }
    }
}

impl<T: FloatingPoint> Mul<T> for Matrix2x2<T> {
    type Output = Self;
    fn mul(self, rhs: T) -> Self::Output {
        Self {
            data: [
                [self.data[0][0] * rhs, self.data[0][1] * rhs],
                [self.data[1][0] * rhs, self.data[1][1] * rhs],
            ],
        }
    }
}

/// Multiplications
impl<T: FloatingPoint> Mul<Vector2<T>> for Matrix2x2<T> {
    type Output = Vector2<T>;
    fn mul(self, rhs: Vector2<T>) -> Self::Output {
        Vector2::new(
            self.data[0][0] * rhs.x + self.data[0][1] * rhs.y,
            self.data[1][0] * rhs.x + self.data[1][1] * rhs.y,
        )
    }
}

impl<T: FloatingPoint> Mul<Matrix2x2<T>> for Vector2<T> {
    type Output = Vector2<T>;
    fn mul(self, rhs: Matrix2x2<T>) -> Self::Output {
        Vector2::new(
            self.x * rhs.data[0][0] + self.y * rhs.data[1][0],
            self.x * rhs.data[0][1] + self.y * rhs.data[1][1],
        )
    }
}

impl<T: FloatingPoint> Mul<Matrix2x2<T>> for Matrix2x2<T> {
    type Output = Matrix2x2<T>;
    fn mul(self, rhs: Matrix2x2<T>) -> Self::Output {
        Self {
            data: [
                [
                    self.data[0][0] * rhs.data[0][0] + self.data[0][1] * rhs.data[1][0],
                    self.data[0][0] * rhs.data[0][1] + self.data[0][1] * rhs.data[1][1],
                ],
                [
                    self.data[1][0] * rhs.data[0][0] + self.data[1][1] * rhs.data[1][0],
                    self.data[1][0] * rhs.data[0][1] + self.data[1][1] * rhs.data[1][1],
                ],
            ],
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Matrix4x4<T: FloatingPoint = f32> {
    pub data: [[T; 4]; 4],
}

impl<T: FloatingPoint + std::ops::Neg<Output = T>> Matrix4x4<T> {
    pub fn new(data: [[T; 4]; 4]) -> Self {
        Self { data }
    }

    pub fn from_rows(r0: [T; 4], r1: [T; 4], r2: [T; 4], r3: [T; 4]) -> Self {
        Self {
            data: [r0, r1, r2, r3],
        }
    }

    pub fn from_columns(c0: [T; 4], c1: [T; 4], c2: [T; 4], c3: [T; 4]) -> Self {
        let mut data = [[T::zero(); 4]; 4];
        for i in 0..4 {
            data[i][0] = c0[i];
            data[i][1] = c1[i];
            data[i][2] = c2[i];
            data[i][3] = c3[i];
        }
        Self { data }
    }

    pub fn zero() -> Self {
        Self {
            data: [[T::zero(); 4]; 4],
        }
    }

    pub fn one() -> Self {
        Self {
            data: [[T::one(); 4]; 4],
        }
    }

    pub fn identity() -> Self {
        let mut m = Self::zero();
        for i in 0..4 {
            m.data[i][i] = T::one();
        }
        m
    }

    pub fn swap_rows(&mut self, r1: usize, r2: usize) {
        self.data.swap(r1, r2);
    }

    pub fn scale_row(&mut self, row: usize, scalar: T) {
        for j in 0..4 {
            self.data[row][j] = self.data[row][j] * scalar;
        }
    }

    pub fn add_row_multiple(&mut self, target: usize, source: usize, scalar: T) {
        for j in 0..4 {
            self.data[target][j] = self.data[target][j] + self.data[source][j] * scalar;
        }
    }

    pub fn transpose(&self) -> Self {
        let mut result = [[T::zero(); 4]; 4];
        for i in 0..4 {
            for j in 0..4 {
                result[j][i] = self.data[i][j];
            }
        }
        Self { data: result }
    }

    pub fn determinant(&self) -> T {
        let m = &self.data;

        // Helper function to compute 3x3 determinant from row/col indices
        let det3 = |r0: usize, c0: usize, r1: usize, c1: usize, r2: usize, c2: usize,
                    r3: usize, c3: usize, r4: usize, c4: usize, r5: usize, c5: usize,
                    r6: usize, c6: usize, r7: usize, c7: usize, r8: usize, c8: usize| {
            m[r0][c0] * (m[r4][c4] * m[r8][c8] - m[r5][c5] * m[r7][c7])
                - m[r1][c1] * (m[r3][c3] * m[r8][c8] - m[r5][c5] * m[r6][c6])
                + m[r2][c2] * (m[r3][c3] * m[r7][c7] - m[r4][c4] * m[r6][c6])
        };

        // Expand by first row
        m[0][0] * det3(1, 1, 1, 2, 1, 3, 2, 1, 2, 2, 2, 3, 3, 1, 3, 2, 3, 3)
            - m[0][1] * det3(1, 0, 1, 2, 1, 3, 2, 0, 2, 2, 2, 3, 3, 0, 3, 2, 3, 3)
            + m[0][2] * det3(1, 0, 1, 1, 1, 3, 2, 0, 2, 1, 2, 3, 3, 0, 3, 1, 3, 3)
            - m[0][3] * det3(1, 0, 1, 1, 1, 2, 2, 0, 2, 1, 2, 2, 3, 0, 3, 1, 3, 2)
    }
}

// From / Into conversions
impl<T: FloatingPoint> From<[[T; 4]; 4]> for Matrix4x4<T> {
    fn from(data: [[T; 4]; 4]) -> Self {
        Self { data }
    }
}
impl<T: FloatingPoint> From<Matrix4x4<T>> for [[T; 4]; 4] {
    fn from(m: Matrix4x4<T>) -> Self {
        m.data
    }
}

// Arithmetic
impl<T: FloatingPoint> Add for Matrix4x4<T> {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        let mut result = [[T::zero(); 4]; 4];
        for i in 0..4 {
            for j in 0..4 {
                result[i][j] = self.data[i][j] + rhs.data[i][j];
            }
        }
        Self { data: result }
    }
}

impl<T: FloatingPoint> Sub for Matrix4x4<T> {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        let mut result = [[T::zero(); 4]; 4];
        for i in 0..4 {
            for j in 0..4 {
                result[i][j] = self.data[i][j] - rhs.data[i][j];
            }
        }
        Self { data: result }
    }
}

impl<T: FloatingPoint> Mul<T> for Matrix4x4<T> {
    type Output = Self;
    fn mul(self, rhs: T) -> Self {
        let mut result = [[T::zero(); 4]; 4];
        for i in 0..4 {
            for j in 0..4 {
                result[i][j] = self.data[i][j] * rhs;
            }
        }
        Self { data: result }
    }
}

impl<T: FloatingPoint> Mul for Matrix4x4<T> {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self {
        let mut result = [[T::zero(); 4]; 4];
        for i in 0..4 {
            for j in 0..4 {
                for k in 0..4 {
                    result[i][j] = result[i][j] + self.data[i][k] * rhs.data[k][j];
                }
            }
        }
        Self { data: result }
    }
}

impl<T: FloatingPoint> Matrix4x4<T> {
    /// Get a row as [T; 4]
    pub fn row(&self, index: usize) -> [T; 4] {
        assert!(index < 4, "row index out of bounds");
        self.data[index]
    }

    /// Set a row
    pub fn set_row(&mut self, index: usize, row: [T; 4]) {
        assert!(index < 4, "row index out of bounds");
        self.data[index] = row;
    }

    /// Get a column as [T; 4]
    pub fn column(&self, index: usize) -> [T; 4] {
        assert!(index < 4, "column index out of bounds");
        let mut col = [T::zero(); 4];
        for r in 0..4 {
            col[r] = self.data[r][index];
        }
        col
    }

    /// Set a column
    pub fn set_column(&mut self, index: usize, col: [T; 4]) {
        assert!(index < 4, "column index out of bounds");
        for r in 0..4 {
            self.data[r][index] = col[r];
        }
    }

    pub fn trace(&self) -> T {
        let mut s = T::zero();
        for i in 0..4 {
            s = s + self.data[i][i];
        }
        s
    }
}

impl<T: FloatingPoint> Index<usize> for Matrix4x4<T> {
    type Output = [T; 4];

    fn index(&self, row: usize) -> &Self::Output {
        assert!(row < 4, "row index out of bounds");
        &self.data[row]
    }
}

impl<T: FloatingPoint> IndexMut<usize> for Matrix4x4<T> {
    fn index_mut(&mut self, row: usize) -> &mut Self::Output {
        assert!(row < 4, "row index out of bounds");
        &mut self.data[row]
    }
}

pub struct RowIter<'a, T: FloatingPoint> {
    data: &'a [[T; 4]; 4],
    index: usize,
}

pub struct ColumnIter<'a, T: FloatingPoint> {
    data: &'a [[T; 4]; 4],
    index: usize,
}

impl<T: FloatingPoint> Matrix4x4<T> {
    pub fn rows(&self) -> RowIter<'_, T> {
        RowIter {
            data: &self.data,
            index: 0,
        }
    }

    pub fn columns(&self) -> ColumnIter<'_, T> {
        ColumnIter {
            data: &self.data,
            index: 0,
        }
    }
}

impl<'a, T: FloatingPoint> Iterator for RowIter<'a, T> {
    type Item = &'a [T; 4];

    fn next(&mut self) -> Option<Self::Item> {
        if self.index < 4 {
            let row = &self.data[self.index];
            self.index += 1;
            Some(row)
        } else {
            None
        }
    }
}

impl<'a, T: FloatingPoint> Iterator for ColumnIter<'a, T> {
    type Item = [T; 4];

    fn next(&mut self) -> Option<Self::Item> {
        if self.index < 4 {
            let mut col = [T::zero(); 4];
            for r in 0..4 {
                col[r] = self.data[r][self.index];
            }
            self.index += 1;
            Some(col)
        } else {
            None
        }
    }
}

pub struct RowIterMut<'a, T: FloatingPoint> {
    data: &'a mut [[T; 4]; 4],
    index: usize,
}

pub struct ColumnIterMut<'a, T: FloatingPoint> {
    data: &'a mut [[T; 4]; 4],
    index: usize,
}

impl<T: FloatingPoint> Matrix4x4<T> {
    pub fn rows_mut(&mut self) -> RowIterMut<'_, T> {
        RowIterMut {
            data: &mut self.data,
            index: 0,
        }
    }

    pub fn columns_mut(&mut self) -> ColumnIterMut<'_, T> {
        ColumnIterMut {
            data: &mut self.data,
            index: 0,
        }
    }
}

impl<'a, T: FloatingPoint> Iterator for RowIterMut<'a, T> {
    type Item = &'a mut [T; 4];

    fn next(&mut self) -> Option<Self::Item> {
        if self.index < 4 {
            let row = unsafe {
                // Rust’s borrow checker won’t allow mutable aliasing without unsafe.
                // We know rows are disjoint, so this is safe.
                &mut *(&mut self.data[self.index] as *mut [T; 4])
            };
            self.index += 1;
            Some(row)
        } else {
            None
        }
    }
}

impl<'a, T: FloatingPoint> Iterator for ColumnIterMut<'a, T> {
    type Item = ColumnMut<'a, T>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index < 4 {
            let col_index = self.index;
            self.index += 1;
            // SAFETY: Each call to next() returns a different column,
            // so we won't have overlapping mutable references
            let data = unsafe { &mut *(self.data as *mut [[T; 4]; 4]) };
            Some(ColumnMut {
                data,
                col: col_index,
            })
        } else {
            None
        }
    }
}

/// Helper wrapper for mutable column access
pub struct ColumnMut<'a, T: FloatingPoint> {
    data: &'a mut [[T; 4]; 4],
    col: usize,
}

impl<'a, T: FloatingPoint> ColumnMut<'a, T> {
    pub fn get(&self) -> [T; 4] {
        let mut col = [T::zero(); 4];
        for r in 0..4 {
            col[r] = self.data[r][self.col];
        }
        col
    }

    pub fn set(&mut self, values: [T; 4]) {
        for r in 0..4 {
            self.data[r][self.col] = values[r];
        }
    }
}

impl<'a, T: FloatingPoint> IntoIterator for &'a Matrix4x4<T> {
    type Item = &'a [T; 4];
    type IntoIter = std::slice::Iter<'a, [T; 4]>;

    fn into_iter(self) -> Self::IntoIter {
        self.data.iter()
    }
}

impl<'a, T: FloatingPoint> IntoIterator for &'a mut Matrix4x4<T> {
    type Item = &'a mut [T; 4];
    type IntoIter = std::slice::IterMut<'a, [T; 4]>;

    fn into_iter(self) -> Self::IntoIter {
        self.data.iter_mut()
    }
}

impl<T: FloatingPoint> IntoIterator for Matrix4x4<T> {
    type Item = [T; 4];
    type IntoIter = std::array::IntoIter<[T; 4], 4>; // Be explicit about the array IntoIter

    fn into_iter(self) -> Self::IntoIter {
        self.data.into_iter()
    }
}

/// --- inverse() via adjugate / determinant
impl<T> Matrix4x4<T>
where
    T: FloatingPoint + Neg<Output = T> + Div<Output = T> + AddAssign + SubAssign + MulAssign,
{
    /// Compute the inverse. Returns None if matrix is singular (determinant == 0).
    /// Note: numeric stability is basic here; for production consider LU decomposition.
    pub fn inverse(&self) -> Option<Self> {
        // compute determinant
        let det = self.determinant();
        if det == T::zero() {
            return None;
        }

        // compute matrix of cofactors (3x3 minors with sign)
        let mut cof = [[T::zero(); 4]; 4];
        for r in 0..4 {
            for c in 0..4 {
                // build 3x3 minor for element (r,c)
                let mut minor = [[T::zero(); 3]; 3];
                let mut rr = 0;
                for i in 0..4 {
                    if i == r { continue; }
                    let mut cc = 0;
                    for j in 0..4 {
                        if j == c { continue; }
                        minor[rr][cc] = self.data[i][j];
                        cc += 1;
                    }
                    rr += 1;
                }
                // determinant of 3x3 minor
                let m = &minor;
                let det3 = m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
                    - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
                    + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0]);
                // cofactor sign
                let sign = if ((r + c) % 2) == 0 { T::one() } else { -T::one() };
                cof[r][c] = sign * det3;
            }
        }

        // adjugate = transpose(cofactor matrix)
        let mut adj = [[T::zero(); 4]; 4];
        for r in 0..4 {
            for c in 0..4 {
                adj[c][r] = cof[r][c];
            }
        }

        // divide adjugate by determinant
        let mut inv = [[T::zero(); 4]; 4];
        for r in 0..4 {
            for c in 0..4 {
                inv[r][c] = adj[r][c] / det;
            }
        }

        Some(Matrix4x4::new(inv))
    }
}

/// --- Display
impl<T> fmt::Display for Matrix4x4<T>
where
    T: FloatingPoint + fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Matrix4x4 [")?;
        for r in 0..4 {
            write!(f, "  [")?;
            for c in 0..4 {
                if c < 3 {
                    write!(f, "{}, ", self.data[r][c])?;
                } else {
                    write!(f, "{}", self.data[r][c])?;
                }
            }
            writeln!(f, "],")?;
        }
        write!(f, "]")
    }
}

/// --- ColumnMut view with Index/IndexMut to make columns mutable like rows
impl<'a, T: FloatingPoint> ColumnMut<'a, T> {
    fn new(data: &'a mut [[T; 4]; 4], col: usize) -> Self {
        assert!(col < 4, "column index out of bounds");
        Self { data, col }
    }
}

impl<'a, T: FloatingPoint> Index<usize> for ColumnMut<'a, T> {
    type Output = T;
    fn index(&self, idx: usize) -> &Self::Output {
        assert!(idx < 4, "index out of bounds");
        &self.data[idx][self.col]
    }
}
impl<'a, T: FloatingPoint> IndexMut<usize> for ColumnMut<'a, T> {
    fn index_mut(&mut self, idx: usize) -> &mut Self::Output {
        assert!(idx < 4, "index out of bounds");
        &mut self.data[idx][self.col]
    }
}

/// --- Algebraic traits in-place (AddAssign, SubAssign, MulAssign<T>) and Neg
impl<T> AddAssign for Matrix4x4<T>
where
    T: FloatingPoint + AddAssign + Copy,
{
    fn add_assign(&mut self, rhs: Self) {
        for r in 0..4 {
            for c in 0..4 {
                self.data[r][c] += rhs.data[r][c];
            }
        }
    }
}

impl<T> SubAssign for Matrix4x4<T>
where
    T: FloatingPoint + SubAssign + Copy,
{
    fn sub_assign(&mut self, rhs: Self) {
        for r in 0..4 {
            for c in 0..4 {
                self.data[r][c] -= rhs.data[r][c];
            }
        }
    }
}

impl<T> MulAssign<T> for Matrix4x4<T>
where
    T: FloatingPoint + MulAssign + Copy,
{
    fn mul_assign(&mut self, rhs: T) {
        for r in 0..4 {
            for c in 0..4 {
                self.data[r][c] *= rhs;
            }
        }
    }
}

impl<T> Neg for Matrix4x4<T>
where
    T: FloatingPoint + Neg<Output = T> + Copy,
{
    type Output = Self;
    fn neg(self) -> Self::Output {
        let mut out = [[T::zero(); 4]; 4];
        for r in 0..4 {
            for c in 0..4 {
                out[r][c] = -self.data[r][c];
            }
        }
        Matrix4x4::new(out)
    }
}

/// columns_mut now yields ColumnMut<'_, T>
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
        let m = Matrix3x3::from_rows([1.0f32, 2.0, 3.0], [4.0, 5.0, 6.0], [7.0, 8.0, 9.0]);

        assert_eq!(m.row(0), [1.0, 2.0, 3.0]);
        assert_eq!(m.column(1), [2.0, 5.0, 8.0]);

        let z = Matrix3x3::<f32>::zero();
        assert_eq!(z, Matrix3x3::new([[0.0; 3]; 3]));

        let o = Matrix3x3::<f32>::one();
        assert_eq!(o, Matrix3x3::new([[1.0; 3]; 3]));

        let id = Matrix3x3::<f32>::identity();
        assert_eq!(
            id,
            Matrix3x3::new([[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]])
        );
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
            Matrix3x3::new([[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]])
        );
    }

    #[test]
    fn test_matrix_add_sub_mul() {
        let a = Matrix3x3::from_rows([1.0, 2.0, 3.0], [4.0, 5.0, 6.0], [7.0, 8.0, 9.0]);
        let b = Matrix3x3::from_rows([9.0, 8.0, 7.0], [6.0, 5.0, 4.0], [3.0, 2.0, 1.0]);

        let sum = a + b;
        assert_eq!(sum.row(0), [10.0, 10.0, 10.0]);

        let diff = a - b;
        assert_eq!(diff.row(2), [4.0, 6.0, 8.0]);

        let scaled = a * 2.0;
        assert_eq!(scaled.row(1), [8.0, 10.0, 12.0]);
    }

    #[test]
    fn test_matrix_vector_mul() {
        let m = Matrix3x3::from_rows([1.0f32, 2.0, 3.0], [4.0, 5.0, 6.0], [7.0, 8.0, 9.0]);

        let v = Vector3::new(1.0f32, 1.0f32, 1.0f32);
        let result = m * v;

        // Row sums: [6, 15, 24]
        assert_eq!(result, Vector3::new(6.0, 15.0, 24.0));
    }

    #[test]
    fn test_vector_matrix_mul() {
        let m = Matrix3x3::from_rows([1.0f32, 2.0, 3.0], [4.0, 5.0, 6.0], [7.0, 8.0, 9.0]);

        let v = Vector3::new(1.0f32, 1.0f32, 1.0f32);
        let result = v * m;

        // Column sums: [12, 15, 18]
        assert_eq!(result, Vector3::new(12.0, 15.0, 18.0));
    }

    #[test]
    fn test_matrix_matrix_mul() {
        let a = Matrix3x3::from_rows([1.0f32, 2.0, 3.0], [4.0, 5.0, 6.0], [7.0, 8.0, 9.0]);

        let b = Matrix3x3::from_rows([9.0f32, 8.0, 7.0], [6.0, 5.0, 4.0], [3.0, 2.0, 1.0]);

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

    #[test]
    fn test_constructors() {
        let m = Matrix2x2::from_rows([1.0, 2.0], [3.0, 4.0]);
        assert_eq!(m.data, [[1.0, 2.0], [3.0, 4.0]]);
    }

    #[test]
    fn test_identity_matrix2x2() {
        let m = Matrix2x2::<f32>::identity();
        assert_eq!(m.data, [[1.0, 0.0], [0.0, 1.0]]);
    }

    #[test]
    fn test_row_ops() {
        let mut m = Matrix2x2::from_rows([1.0, 2.0], [3.0, 4.0]);
        m.swap_rows(0, 1);
        assert_eq!(m.data, [[3.0, 4.0], [1.0, 2.0]]);
        m.scale_row(0, 2.0);
        assert_eq!(m.data, [[6.0, 8.0], [1.0, 2.0]]);
        m.add_row_multiple(1, 0, 0.5);
        assert_eq!(m.data, [[6.0, 8.0], [4.0, 6.0]]);
    }

    #[test]
    fn test_arithmetic_matrix2x2() {
        let a = Matrix2x2::from_rows([1.0, 2.0], [3.0, 4.0]);
        let b = Matrix2x2::from_rows([5.0, 6.0], [7.0, 8.0]);
        let c = a + b;
        assert_eq!(c.data, [[6.0, 8.0], [10.0, 12.0]]);
    }

    #[test]
    fn test_mul_matrix_vector() {
        let m = Matrix2x2::from_rows([1.0, 2.0], [3.0, 4.0]);
        let v = Vector2::new(1.0, 1.0);
        let result = m * v;
        assert_eq!(result, Vector2::new(3.0, 7.0));
    }

    #[test]
    fn test_determinant_matrix2x2() {
        let m = Matrix2x2::from_rows([1.0, 2.0], [3.0, 4.0]);
        assert_eq!(m.determinant(), -2.0);
    }

    #[test]
    fn test_identity_matrix4x4() {
        let m = Matrix4x4::<f32>::identity();
        assert_eq!(m.data[0][0], 1.0);
        assert_eq!(m.data[1][1], 1.0);
        assert_eq!(m.data[2][2], 1.0);
        assert_eq!(m.data[3][3], 1.0);
    }

    #[test]
    fn test_serialization_bincode_matrix4x4() {
        let m = Matrix4x4::<f32>::identity();
        let encoded = bincode::serialize(&m.data).unwrap();
        let decoded: [[f32; 4]; 4] = bincode::deserialize(&encoded).unwrap();
        assert_eq!(decoded, m.data);
    }

    #[test]
    fn test_transpose_matrix4x4() {
        let m = Matrix4x4::from_rows(
            [1.0, 2.0, 3.0, 4.0],
            [5.0, 6.0, 7.0, 8.0],
            [9.0, 10.0, 11.0, 12.0],
            [13.0, 14.0, 15.0, 16.0],
        );
        let t = m.transpose();
        assert_eq!(t.data[0][1], 5.0);
        assert_eq!(t.data[1][0], 2.0);
    }

    #[test]
    fn test_row_column_access_matrix4x4() {
        let mut m = Matrix4x4::identity();

        // Replace row 0
        m.set_row(0, [1.0, 2.0, 3.0, 4.0]);
        assert_eq!(m.row(0), [1.0, 2.0, 3.0, 4.0]);

        // Replace column 1
        m.set_column(1, [5.0, 6.0, 7.0, 8.0]);
        assert_eq!(m.column(1), [5.0, 6.0, 7.0, 8.0]);
    }

    #[test]
    fn test_index_access_matrix4x4() {
        let mut m = Matrix4x4::identity();

        // direct element access
        assert_eq!(m[0][0], 1.0);
        assert_eq!(m[1][0], 0.0);

        // mutate via indexing
        m[2][3] = 42.0;
        assert_eq!(m[2][3], 42.0);
    }

    #[test]
    fn test_iterators_matrix4x4() {
        let m = Matrix4x4::from_rows(
            [1.0, 2.0, 3.0, 4.0],
            [5.0, 6.0, 7.0, 8.0],
            [9.0, 10.0, 11.0, 12.0],
            [13.0, 14.0, 15.0, 16.0],
        );

        // Iterate rows
        let rows: Vec<[f32; 4]> = m.rows().map(|r| *r).collect();
        assert_eq!(rows[0], [1.0, 2.0, 3.0, 4.0]);

        // Iterate columns
        let cols: Vec<[f32; 4]> = m.columns().collect();
        assert_eq!(cols[1], [2.0, 6.0, 10.0, 14.0]);
    }

    #[test]
    fn test_mutable_iterators_matrix4x4() {
        let mut m = Matrix4x4::identity();

        // Mutate rows
        for row in m.rows_mut() {
            row[0] = 5.0;
        }
        assert_eq!(m[1][0], 5.0);

        // Mutate columns
        for mut col in m.columns_mut() {
            let mut values = col.get();
            values[3] = 42.0;
            col.set(values);
        }
        assert_eq!(m[3][2], 42.0);
    }

    #[test]
    fn test_into_iter_matrix4x4() {
        let m = Matrix4x4::from_rows(
            [1.0, 2.0, 3.0, 4.0],
            [5.0, 6.0, 7.0, 8.0],
            [9.0, 10.0, 11.0, 12.0],
            [13.0, 14.0, 15.0, 16.0],
        );

        // Immutable borrow
        let sum: f32 = (&m).into_iter().map(|row| row[0]).sum();
        assert_eq!(sum, 1.0 + 5.0 + 9.0 + 13.0);

        // Mutable borrow
        let mut m2 = m.clone();
        for row in &mut m2 {
            row[0] = 0.0;
        }
        assert_eq!(m2[0][0], 0.0);
        assert_eq!(m2[1][0], 0.0);

        // By value
        let rows: Vec<[f32; 4]> = m.into_iter().collect();
        assert_eq!(rows[2], [9.0, 10.0, 11.0, 12.0]);
    }

    fn approx_eq<T: FloatingPoint>(a: T, b: T, eps: T) -> bool {
        (a - b).abs() <= eps
    }

    #[test]
    fn test_zero_and_one() {
        let zero = Matrix4x4::<f32>::zero();
        let one = Matrix4x4::<f32>::one();
        assert!(zero.data.iter().all(|row| row.iter().all(|&x| x == 0.0)));
        assert!(one.data.iter().all(|row| row.iter().all(|&x| x == 1.0)));
    }

    #[test]
    fn test_trace() {
        let m = Matrix4x4::from_rows(
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 2.0, 0.0, 0.0],
            [0.0, 0.0, 3.0, 0.0],
            [0.0, 0.0, 0.0, 4.0],
        );
        let trace: f32 = (0..4).map(|i| m[i][i]).sum();
        assert_eq!(trace, 10.0);
    }

    #[test]
    fn test_determinant_identity_and_zero() {
        let id = Matrix4x4::<f32>::identity();
        assert_eq!(id.determinant(), 1.0);

        let zero = Matrix4x4::<f32>::zero();
        assert_eq!(zero.determinant(), 0.0);
    }

    #[test]
    fn test_determinant_known_matrix() {
        let m = Matrix4x4::from_rows(
            [1.0, 2.0, 3.0, 4.0],
            [5.0, 6.0, 7.0, 8.0],
            [2.0, 6.0, 4.0, 8.0],
            [3.0, 1.0, 1.0, 2.0],
        );

        // precomputed using numpy.linalg.det
        let det = 72.0;
        assert!(approx_eq(m.determinant(), det, 1e-5));
    }

    #[test]
    fn test_transpose_invariants() {
        let id = Matrix4x4::<f32>::identity();
        assert_eq!(id.transpose(), id);

        let m = Matrix4x4::from_rows(
            [0.0, 1.0, 2.0, 3.0],
            [4.0, 5.0, 6.0, 7.0],
            [8.0, 9.0, 10.0, 11.0],
            [12.0, 13.0, 14.0, 15.0],
        );
        let t = m.transpose();
        assert_eq!(t.transpose(), m);
    }

    #[test]
    fn test_transpose_multiplication_property() {
        let a = Matrix4x4::from_rows(
            [1.0, 2.0, 3.0, 4.0],
            [5.0, 6.0, 7.0, 8.0],
            [9.0, 10.0, 11.0, 12.0],
            [13.0, 14.0, 15.0, 16.0],
        );
        let b = Matrix4x4::from_rows(
            [2.0, 0.0, 1.0, 0.0],
            [0.0, 2.0, 0.0, 1.0],
            [1.0, 0.0, 2.0, 0.0],
            [0.0, 1.0, 0.0, 2.0],
        );

        let lhs = (a.clone() * b.clone()).transpose();
        let rhs = b.transpose() * a.transpose();

        for i in 0..4 {
            for j in 0..4 {
                assert!(approx_eq(lhs[i][j], rhs[i][j], 1e-5));
            }
        }
    }

    #[test]
    fn test_scalar_multiplication_edge_cases() {
        let m = Matrix4x4::<f32>::identity();

        let zero_scaled = m.clone() * 0.0;
        assert!(zero_scaled
            .data
            .iter()
            .all(|row| row.iter().all(|&x| x == 0.0)));

        let one_scaled = m.clone() * 1.0;
        assert_eq!(m, one_scaled);
    }

    #[test]
    fn test_row_column_accessors() {
        let mut m = Matrix4x4::<f32>::identity();
        m.set_row(0, [1.0, 2.0, 3.0, 4.0]);
        assert_eq!(m.row(0), [1.0, 2.0, 3.0, 4.0]);

        m.set_column(1, [5.0, 6.0, 7.0, 8.0]);
        assert_eq!(m.column(1), [5.0, 6.0, 7.0, 8.0]);
    }

    #[test]
    fn test_serialization_bincode_roundtrip() {
        let m = Matrix4x4::<f32>::identity();
        let encoded = bincode::serialize(&m.data).unwrap();
        let decoded: [[f32; 4]; 4] = bincode::deserialize(&encoded).unwrap();
        assert_eq!(decoded, m.data);
    }
}
