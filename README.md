# Rust Text Editor

A Windows-first, Rust-native IDE with a VS Code-inspired layout.

## Product direction

- Native application; no Electron/Chromium
- Modular Rust Cargo workspace
- Windows 11 first; Linux and macOS later
- Full IDE: LSP, debugging, integrated terminal, Git, and project navigation
- Extensions after core IDE functionality is stable

## Development status

**Group 1 — Foundation (initial scaffold).** Windows compile and runtime verification are still required.

## Prerequisites (Windows)

- Rust stable and Cargo (MSVC target)
- Visual Studio 2022 Build Tools with Desktop development with C++
- Windows 10/11 SDK (including legacy HLSL compiler tools when required by GPUI)

## Commands

```powershell
cargo fmt --all -- --check
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo run -p rust-text-editor
```

## Architecture

The UI is an adapter. Document, buffer, and editor-domain crates must not depend on GPUI or Windows APIs.

See [Architecture](docs/architecture.md) and [Roadmap](docs/roadmap.md).

## Release policy

Development groups are internal milestones. Target v1.0.0 only after IDE acceptance tests and release qualification.
