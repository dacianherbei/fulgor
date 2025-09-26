# Fulgor Project Structure

## Directory Layout
```
fulgor/
├── Cargo.toml                    # Workspace root
├── README.md
├── .gitignore
├── crates/
│   ├── fulgor/                   # Main namespace crate
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   └── lib.rs            # Re-exports all modules
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