# Cargo Test

Run Rust tests for $ARGUMENTS in the Tauri backend.

## Process

1. **Identify scope**: Determine which module or test to run based on the argument
2. **Run tests**: Execute `cargo test` with the appropriate filter
3. **Analyze failures**: If tests fail, read the relevant source and test code
4. **Fix issues**: Suggest or implement fixes for failing tests

## Commands

```bash
# Run all tests
source ~/.cargo/env && cargo test --manifest-path src-tauri/Cargo.toml

# Run tests for a specific module
source ~/.cargo/env && cargo test --manifest-path src-tauri/Cargo.toml <module_name>

# Run with output
source ~/.cargo/env && cargo test --manifest-path src-tauri/Cargo.toml -- --nocapture
```

## Project Structure

- Rust source: `src-tauri/src/core/` (coordinates, easing, keyframe, track, timeline, evaluator, generators, project, capture, input, permissions, encoder)
- Tests are inline `#[cfg(test)] mod tests` within each module
- Key dependencies: serde, serde_json, uuid, thiserror
