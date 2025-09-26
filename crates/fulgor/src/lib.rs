//! # Fulgor - Revolutionary Node-Based IDE
//!
//! Fulgor provides a complete ecosystem for visual programming with zero-overhead compilation.

/// Node-based execution engine
pub use nexus as nexus;

/// LLVM code generation and optimization
pub use forge as forge;

/// Advanced splats rendering engine
pub use lights as lights;

/// Crypto marketplace and monetization
pub use agora as agora;

/// UI component library for visual interfaces
pub use optical as optical;

/// Main application coordinator and workflow engine
pub use atrium as atrium;

// Re-export commonly used types for convenience
// pub use nexus::{NodeDefinition, WorkflowEngine, ExecutionMode};
// pub use lights::{SplatsRenderer, RenderingPipeline};
// pub use optical::{Widget, Layout, Component};

/// Common error type used across all Fulgor components
// pub use anyhow::{Error, Result};

/// Prelude module for convenient imports
pub mod prelude {
    pub use super::{nexus, forge, lights, agora, optical, atrium};
    // pub use nexus::{NodeDefinition, WorkflowEngine};
    // pub use lights::SplatsRenderer;
    // pub use optical::{Widget, Component};
    // pub use anyhow::{Error, Result};
}