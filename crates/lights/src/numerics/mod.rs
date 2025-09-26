// src/numerics/mod.rs
// Top-level numerics module. Exposes a `types` namespace with submodules.

#![allow(dead_code)]

pub mod types {
    // The submodules live in src/numerics/types/*.rs
    pub mod vector;
    pub mod matrix;
    pub mod point;
    pub mod traits;
}
