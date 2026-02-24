# Debug Assistant

Help me debug the issue with $ARGUMENTS in LazyRec.

## Process

1. Understand the issue description
2. Determine if it's a frontend (React/TS) or backend (Rust/Tauri) issue
3. Locate relevant files and analyze code flow
4. Identify root cause and implement fix
5. Verify with `cargo test` (Rust) or `tsc --noEmit` (TypeScript)

## Project-Specific Debugging

### Frontend (src/)
- Entry: `src/main.tsx` → `src/App.tsx` (3 screens: Welcome, Recording, Editor)
- Styles: `src/App.css`
- Tauri IPC: `invoke()` from `@tauri-apps/api/core`
- Dev server: Vite on port 1420

### Backend (src-tauri/src/)
- Entry: `main.rs` → `lib.rs` (Tauri setup + commands)
- Core modules: `src-tauri/src/core/` (coordinates, easing, keyframe, track, timeline, evaluator, generators, project, capture, input, permissions, encoder)
- Run tests: `source ~/.cargo/env && cargo test --manifest-path src-tauri/Cargo.toml`
- Check types: `source ~/.cargo/env && cargo check --manifest-path src-tauri/Cargo.toml`

### Common Issues
- Rust compile errors: Check `cargo check` output
- Tauri IPC failures: Verify command is registered in lib.rs and capabilities/default.json
- Frontend type errors: Run `npx tsc --noEmit`
- Coordinate issues: All internal coords use NormalizedPoint (0-1 range, top-left origin)
