# Tauri Dev & Build

Run or build the Tauri application: $ARGUMENTS

## Commands

```bash
# Development (frontend HMR + Rust backend)
npm run tauri dev

# Production build
npm run tauri build

# Frontend only (Vite dev server on :1420)
npm run dev

# Frontend build only
npm run build

# Type check frontend
npx tsc --noEmit
```

## Architecture

- **Frontend**: React 19 + TypeScript 5.8 + Vite 7 (port 1420)
- **Backend**: Rust + Tauri v2 (IPC commands in src-tauri/src/lib.rs)
- **Config**: src-tauri/tauri.conf.json (window 1200x800, bundle targets all platforms)
- **Capabilities**: src-tauri/capabilities/default.json (core + opener permissions)

## Troubleshooting

- If Rust changes don't reflect: `cargo clean --manifest-path src-tauri/Cargo.toml`
- If frontend port conflict: Check tauri.conf.json devUrl and vite.config.ts server.port
- Rust toolchain: `source ~/.cargo/env` before cargo commands
