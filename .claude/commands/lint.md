# Lint Assistant

Analyze and fix code quality issues in $ARGUMENTS.

## Process

1. Determine if checking Rust or TypeScript code
2. Run appropriate linting tools
3. Fix issues and verify

## Rust Linting

```bash
# Clippy (primary linter)
source ~/.cargo/env && cargo clippy --manifest-path src-tauri/Cargo.toml 2>&1

# Format check
source ~/.cargo/env && cargo fmt --manifest-path src-tauri/Cargo.toml -- --check

# Auto-format
source ~/.cargo/env && cargo fmt --manifest-path src-tauri/Cargo.toml
```

## TypeScript Linting

```bash
# Type checking (no eslint configured — use tsc)
npx tsc --noEmit

# Check specific file types
npx tsc --noEmit --pretty
```

## Project Notes

- No ESLint or Prettier configured as project dependencies
- Rust: Use clippy + rustfmt (both available in toolchain)
- TypeScript: Strict mode enabled with noUnusedLocals, noUnusedParameters
- PostToolUse hook auto-runs rustfmt on .rs file edits
