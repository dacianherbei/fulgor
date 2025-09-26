# fulgor — A Modular 3D Rendering Engine Library

This repository contains the Rust library **fulgor**, a modular 3D rendering engine
centered around Gaussian splatting primitives.

## Modules

- **numerics** — Mathematical foundations and templated math types.
- **scene** — Scene graph and entities.
- **renderer_cpu_ref** — CPU-based reference renderer.
- **renderer_gpu_opt** — Optimized GPU renderer.
- **physics** — Physics simulation and interaction.
- **io** — Import/export of assets and formats.
- **tools** — Development and debugging tools.

## Design Goals

- Minimal external dependencies.
- No use of the `glam` library.
- Namespaces group types (`mod.rs`).
- Templated types for flexible precision.
- Instantiate templates only for testing and examples.

## Getting Started

Build the library:

```bash
cargo build
