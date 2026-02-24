# Cargo Check & Clippy

Check Rust code for compilation errors and lint issues in $ARGUMENTS.

## Process

1. **Run cargo check**: Fast compilation check without producing binaries
2. **Run cargo clippy**: Lint for common mistakes and style issues
3. **Analyze warnings**: Review any warnings or errors
4. **Fix issues**: Apply clippy suggestions and resolve compilation errors

## Commands

```bash
# Type check only (fast)
source ~/.cargo/env && cargo check --manifest-path src-tauri/Cargo.toml 2>&1

# Lint with clippy
source ~/.cargo/env && cargo clippy --manifest-path src-tauri/Cargo.toml 2>&1

# Format check (don't write)
source ~/.cargo/env && cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
```

## Project Notes

- Crate name: `lazyrec_lib` (lib target with staticlib, cdylib, rlib)
- Edition: 2021
- Core modules in `src-tauri/src/core/`
- Tauri commands exposed via `src-tauri/src/lib.rs`
