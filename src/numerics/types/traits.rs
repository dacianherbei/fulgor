// src/numerics/types/traits.rs
// Small FloatingPoint trait marker as requested.

#![allow(dead_code)]

/// FloatingPoint is a minimal marker trait for floating point types
/// that we will use in the numerics types.
///
/// Note: We require Copy, PartialOrd and the basic arithmetic ops on Self.
pub trait FloatingPoint:
Copy
+ PartialOrd
+ core::ops::Add<Output = Self>
+ core::ops::Sub<Output = Self>
+ core::ops::Mul<Output = Self>
+ core::ops::Div<Output = Self>
{
}

impl FloatingPoint for f32 {}
impl FloatingPoint for f64 {}
