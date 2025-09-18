// src/numerics/types/matrix.rs
// Minimal placeholder for the matrix module referenced by the layout.
// Kept intentionally tiny — expand later as needed.

#![allow(dead_code)]

use serde::{Serialize, Deserialize};

/// Minimal Matrix3x3 type placeholder for now.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Matrix3x3<T = f32> {
    pub data: [[T; 3]; 3],
}

impl<T> Matrix3x3<T> {
    pub fn new(data: [[T; 3]; 3]) -> Self {
        Self { data }
    }
}

// Generic serde implementations for Matrix3x3
impl<T> Serialize for Matrix3x3<T>
where
    T: Serialize + Copy,
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
    T: Deserialize<'de> + Copy,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let arr = <[[T; 3]; 3]>::deserialize(deserializer)?;
        Ok(Matrix3x3 { data: arr })
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
}