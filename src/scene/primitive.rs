//! Primitive definitions for the `fuller` scene module.
//!
//! Defines `Splat<T>` which is a simple Gaussian-splat-like primitive.
//!
//! The generic parameter `T` is constrained to the project's numeric trait:
//! `numerics::types::traits::FloatingPoint` (defaults to `f32`).
//!
//! Internally the Splat stores compact data using arrays so we do not pull
//! in vector/matrix dependencies here. This keeps the surface small while
//! remaining generic.

use std::fmt;
use std::ops::{Add, Mul};
use crate::numerics::types::traits::FloatingPoint;

/// A simple Splat (Gaussian splat primitive).
///
/// Fields:
/// - `position`: [x, y, z]
/// - `radius`: scalar radius controlling spread
/// - `color`: RGBA 4-tuple in `[0..1]` space
///
/// Generic parameter `T` defaults to `f32` and must implement the project's FloatingPoint trait.
#[derive(Clone, PartialEq)]
pub struct Splat<T: FloatingPoint = f32> {
    pub position: [T; 3],
    pub radius: T,
    pub color: [T; 4],
}

impl<T: FloatingPoint> Splat<T> {
    /// Construct a new splat.
    pub fn new(position: [T; 3], radius: T, color: [T; 4]) -> Self {
        Self {
            position,
            radius,
            color,
        }
    }

    /// Compute a naive squared distance from this splat to a point.
    pub fn squared_distance_to_point(&self, point: [T; 3]) -> T {
        let dx = self.position[0] - point[0];
        let dy = self.position[1] - point[1];
        let dz = self.position[2] - point[2];
        dx * dx + dy * dy + dz * dz
    }

    /// Scale splat uniformly by a scalar. Returns new splat (does not mutate).
    pub fn scale(&self, scalar: T) -> Self {
        Self {
            position: [self.position[0] * scalar, self.position[1] * scalar, self.position[2] * scalar],
            radius: self.radius * scalar,
            color: self.color,
        }
    }

    /// Translate splat by a vector
    pub fn translate(&self, offset: [T; 3]) -> Self {
        Self {
            position: [self.position[0] + offset[0], self.position[1] + offset[1], self.position[2] + offset[2]],
            radius: self.radius,
            color: self.color,
        }
    }
}

/// Component-wise addition of two splats (adds positions, radii, and blends color by addition).
impl<T:FloatingPoint> Add for Splat<T> {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            position: [
                self.position[0] + rhs.position[0],
                self.position[1] + rhs.position[1],
                self.position[2] + rhs.position[2],
            ],
            radius: self.radius + rhs.radius,
            color: [
                self.color[0] + rhs.color[0],
                self.color[1] + rhs.color[1],
                self.color[2] + rhs.color[2],
                self.color[3] + rhs.color[3],
            ],
        }
    }
}

/// Multiply splat by scalar (uniform scale). Implemented for `Splat<T> * T`.
impl<T:FloatingPoint> Mul<T> for Splat<T> {
    type Output = Self;

    fn mul(self, rhs: T) -> Self::Output {
        self.scale(rhs)
    }
}

impl<T> fmt::Debug for Splat<T>
where
    T: FloatingPoint + fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Splat")
            .field(
                "position",
                &format_args!(
                    "[{:.3}, {:.3}, {:.3}]",
                    self.position[0], self.position[1], self.position[2]
                ),
            )
            .field("radius", &format_args!("{:.3}", self.radius))
            .field(
                "color",
                &format_args!(
                    "[{:.3}, {:.3}, {:.3}, {:.3}]",
                    self.color[0], self.color[1], self.color[2], self.color[3]
                ),
            )
            .finish()
    }
}

impl<T:FloatingPoint + std::fmt::Debug> fmt::Display for Splat<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Human readable single-line
        write!(
            f,
            "Splat(pos=[{:.3?},{:.3?},{:.3?}], r={:.3?})",
            self.position[0], self.position[1], self.position[2], self.radius
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splat_basic_ops() {
        let a = Splat::new([0.0f32, 0.0, 0.0], 1.0, [1.0, 0.0, 0.0, 1.0]);
        let b = Splat::new([1.0, 2.0, 3.0], 0.5, [0.0, 1.0, 0.0, 1.0]);

        let c = a.clone() + b.clone();
        assert_eq!(c.position, [1.0, 2.0, 3.0]);
        assert!((c.radius - 1.5).abs() < 1e-6);

        let scaled = b.clone() * 2.0;
        assert_eq!(scaled.radius, 1.0);
        assert_eq!(scaled.position, [2.0, 4.0, 6.0]);

        let d = a.translate([0.5, 0.0, 0.0]);
        assert_eq!(d.position, [0.5, 0.0, 0.0]);
    }
}
