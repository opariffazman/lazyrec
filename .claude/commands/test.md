# Test Assistant

Help with tests for $ARGUMENTS in LazyRec.

## Process

1. Determine if testing Rust backend or React frontend
2. Examine existing test patterns in the codebase
3. Write or fix tests following project conventions
4. Run tests and verify they pass

## Rust Tests (Primary)

Most logic lives in Rust. Tests are inline `#[cfg(test)] mod tests` blocks.

```bash
# All tests
source ~/.cargo/env && cargo test --manifest-path src-tauri/Cargo.toml

# Specific module
source ~/.cargo/env && cargo test --manifest-path src-tauri/Cargo.toml core::easing

# With output
source ~/.cargo/env && cargo test --manifest-path src-tauri/Cargo.toml -- --nocapture
```

### Testable Modules
- `coordinates` — NormalizedPoint math, clamping, interpolation
- `easing` — curve functions (linear, ease in/out, cubic bezier, spring)
- `keyframe` — keyframe value types and interpolation
- `track` — track operations (add/remove/find keyframes)
- `timeline` — timeline with multiple tracks
- `evaluator` — frame state evaluation at arbitrary times
- `generators` — smart zoom, ripple, keystroke generation
- `project` — serialization/deserialization

## React Tests (Future)

No test framework installed yet. When added:
- Vitest recommended (already Vite-based project)
- React Testing Library for component tests
- Test files: `src/*.test.tsx`
