// src/numerics/types/traits.rs
// Small FloatingPoint trait marker as requested.

#![allow(dead_code)]

/// FloatingPoint is a minimal marker trait for floating point types
/// that we will use in the numerics types.
///
/// Note: We require Copy, PartialOrd and the basic arithmetic ops on Self.
pub trait FloatingPoint:
Copy
+ Clone
+ Default
+ PartialEq
+ PartialOrd
+ std::fmt::Debug
+ std::ops::Add<Output = Self>
+ std::ops::Sub<Output = Self>
+ std::ops::Mul<Output = Self>
+ std::ops::Div<Output = Self>
{
    fn zero() -> Self;
    fn one() -> Self;
    fn abs(self) -> Self;
    fn sqrt(self) -> Self;
}

impl FloatingPoint for f32 {
    fn zero() -> Self { 0.0 }
    fn one() -> Self { 1.0 }
    fn abs(self) -> Self { f32::abs(self) }
    fn sqrt(self) -> Self { f32::sqrt(self) }
}

impl FloatingPoint for f64 {
    fn zero() -> Self { 0.0 }
    fn one() -> Self { 1.0 }
    fn abs(self) -> Self { f64::abs(self) }
    fn sqrt(self) -> Self { f64::sqrt(self) }
}
