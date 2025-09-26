# Fulgor Project Structure

## Directory Layout
```
fulgor/
├── Cargo.toml                    # Workspace root
├── README.md
├── .gitignore
├── crates/
│   ├── nexus/
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── execution/
│   │   │   ├── library_integration/
│   │   │   └── optimization/
│   │   └── tests/
│   ├── forge/
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── codegen/
│   │   │   ├── llvm_interface/
│   │   │   └── optimization/
│   │   └── tests/
│   ├── lights/
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── splats/
│   │   │   ├── rendering/
│   │   │   └── gpu/
│   │   └── tests/
│   ├── agora/
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── marketplace/
│   │   │   ├── crypto/
│   │   │   └── monetization/
│   │   └── tests/
│   ├── optical/
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── components/
│   │   │   ├── layout/
│   │   │   └── interaction/
│   │   └── tests/
│   └── atrium/
│       ├── Cargo.toml
│       ├── src/
│       │   ├── lib.rs
│       │   ├── main.rs
│       │   ├── coordination/
│       │   ├── integration/
│       │   └── workflow/
│       └── tests/
├── docs/
├── examples/
└── benches/
```

## Root Cargo.toml (Workspace)
```toml
[workspace]
members = [
    "crates/nexus",
    "crates/forge", 
    "crates/lights",
    "crates/agora",
    "crates/optical",
    "crates/atrium"
]

resolver = "2"

[workspace.package]
version = "0.1.0"
edition = "2021"
authors = ["Your Name <your.email@example.com>"]
license = "MIT OR Apache-2.0"
repository = "https://github.com/your-org/fulgor"
homepage = "https://fulgor.dev"

[workspace.dependencies]
# Shared dependencies across workspace
tokio = { version = "1.0", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
anyhow = "1.0"
thiserror = "1.0"
tracing = "0.1"
tracing-subscriber = "0.3"

# Internal crate dependencies
fulgor-nexus = { path = "crates/nexus" }
fulgor-forge = { path = "crates/forge" }
fulgor-lights = { path = "crates/lights" }
fulgor-agora = { path = "crates/agora" }
fulgor-optical = { path = "crates/optical" }
fulgor-atrium = { path = "crates/atrium" }
```

## crates/nexus/Cargo.toml
```toml
[package]
name = "fulgor-nexus"
version.workspace = true
edition.workspace = true
authors.workspace = true
license.workspace = true
repository.workspace = true
description = "Node-based execution engine for Fulgor IDE"

[dependencies]
# Workspace dependencies
tokio.workspace = true
serde.workspace = true
serde_json.workspace = true
anyhow.workspace = true
thiserror.workspace = true
tracing.workspace = true

# Specific dependencies
toml = "0.8"
uuid = { version = "1.0", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
async-trait = "0.1"
futures = "0.3"

# Optional for library integration
cargo_metadata = { version = "0.18", optional = true }
syn = { version = "2.0", features = ["full"], optional = true }
quote = "1.0"
proc-macro2 = "1.0"

[features]
default = ["library-integration"]
library-integration = ["cargo_metadata", "syn"]

[dev-dependencies]
tokio-test = "0.4"
criterion = "0.5"

[[bench]]
name = "execution_performance"
harness = false
```

## crates/forge/Cargo.toml
```toml
[package]
name = "fulgor-forge"
version.workspace = true
edition.workspace = true
authors.workspace = true
license.workspace = true
repository.workspace = true
description = "LLVM-based code generation for Fulgor IDE"

[dependencies]
# Workspace dependencies
fulgor-nexus.workspace = true
anyhow.workspace = true
thiserror.workspace = true
tracing.workspace = true

# LLVM specific
inkwell = { version = "0.4", features = ["llvm15-0"] }
llvm-sys = "150.0"

# Code generation
tempfile = "3.0"
which = "4.0"

[dev-dependencies]
criterion = "0.5"

[[bench]]
name = "codegen_performance"
harness = false
```

## crates/lights/Cargo.toml
```toml
[package]
name = "fulgor-lights"
version.workspace = true
edition.workspace = true
authors.workspace = true
license.workspace = true
repository.workspace = true
description = "Splats rendering engine for Fulgor"

[dependencies]
# Workspace dependencies
fulgor-nexus.workspace = true
anyhow.workspace = true
thiserror.workspace = true
tracing.workspace = true

# Graphics dependencies
wgpu = "0.18"
winit = "0.29"
pollster = "0.3"
bytemuck = { version = "1.0", features = ["derive"] }
cgmath = "0.18"
image = "0.24"

# Math and utilities
glam = { version = "0.24", features = ["mint"] }
rayon = "1.7"

[dev-dependencies]
criterion = "0.5"
env_logger = "0.10"

[[bench]]
name = "rendering_performance"
harness = false

[[example]]
name = "basic_splats"
path = "examples/basic_splats.rs"
```

## crates/agora/Cargo.toml
```toml
[package]
name = "fulgor-agora"
version.workspace = true
edition.workspace = true
authors.workspace = true
license.workspace = true
repository.workspace = true
description = "Crypto marketplace for Fulgor IDE"

[dependencies]
# Workspace dependencies
fulgor-nexus.workspace = true
fulgor-lights.workspace = true
tokio.workspace = true
serde.workspace = true
serde_json.workspace = true
anyhow.workspace = true
thiserror.workspace = true
tracing.workspace = true

# Crypto and blockchain
ethers = "2.0"
web3 = "0.19"
secp256k1 = "0.27"
sha2 = "0.10"
hex = "0.4"

# HTTP and networking
reqwest = { version = "0.11", features = ["json"] }
url = "2.4"

# Database
sqlx = { version = "0.7", features = ["runtime-tokio-rustls", "postgres", "chrono", "uuid"] }

[dev-dependencies]
tokio-test = "0.4"
```

## crates/optical/Cargo.toml
```toml
[package]
name = "fulgor-optical"
version.workspace = true
edition.workspace = true
authors.workspace = true
license.workspace = true
repository.workspace = true
description = "UI component library for Fulgor IDE"

[dependencies]
# Workspace dependencies
fulgor-lights.workspace = true
serde.workspace = true
anyhow.workspace = true
thiserror.workspace = true

# UI and windowing
winit = "0.29"
wgpu = "0.18"

# Layout and styling
taffy = "0.3"
cosmic-text = "0.9"

# Math and utilities
glam = "0.24"
kurbo = "0.9"

# Input handling
gilrs = "0.10"

[dev-dependencies]
env_logger = "0.10"

[[example]]
name = "button_demo"
path = "examples/button_demo.rs"

[[example]]
name = "layout_demo" 
path = "examples/layout_demo.rs"
```

## crates/atrium/Cargo.toml
```toml
[package]
name = "fulgor-atrium"
version.workspace = true
edition.workspace = true
authors.workspace = true
license.workspace = true
repository.workspace = true
description = "Main GUI coordinator for Fulgor IDE"

[[bin]]
name = "fulgor"
path = "src/main.rs"

[dependencies]
# All workspace crates
fulgor-nexus.workspace = true
fulgor-forge.workspace = true
fulgor-lights.workspace = true
fulgor-agora.workspace = true
fulgor-optical.workspace = true

# Workspace dependencies
tokio.workspace = true
serde.workspace = true
serde_json.workspace = true
anyhow.workspace = true
thiserror.workspace = true
tracing.workspace = true
tracing-subscriber.workspace = true

# Application framework
clap = { version = "4.0", features = ["derive"] }
directories = "5.0"
config = "0.13"

# Async runtime
tokio = { workspace = true, features = ["rt-multi-thread", "macros"] }

[dev-dependencies]
tempfile = "3.0"

[[example]]
name = "minimal_ide"
path = "examples/minimal_ide.rs"
```