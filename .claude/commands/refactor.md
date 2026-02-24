# Code Refactoring Assistant

Refactor $ARGUMENTS in LazyRec following project conventions.

## Process

1. Read the target code and understand its purpose
2. Identify improvement opportunities
3. Implement changes while maintaining behavior
4. Verify with `cargo test` (Rust) or `tsc --noEmit` (TypeScript)

## Project Conventions

### Rust (src-tauri/src/)
- Edition 2021, serde Serialize/Deserialize on all data types
- UUIDs for all track/keyframe identifiers
- NormalizedPoint (0-1 range, top-left origin) for coordinates
- Keyframes always sorted by time within tracks
- thiserror for error types
- Platform-specific code behind trait boundaries

### TypeScript (src/)
- React 19 functional components with hooks
- Strict TypeScript (noUnusedLocals, noUnusedParameters)
- Plain CSS (no CSS-in-JS)
- Tauri v2 IPC for frontend-backend communication

## Refactoring Techniques
- Extract reusable components or Rust modules
- Improve type safety (generics, stricter types)
- Reduce duplication across similar track/keyframe types
- Optimize hot paths (evaluator, interpolation)
