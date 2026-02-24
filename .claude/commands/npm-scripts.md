# NPM Scripts Assistant

Help with NPM scripts: $ARGUMENTS

## Current Scripts (package.json)

```json
{
  "dev": "vite",
  "build": "tsc && vite build",
  "preview": "vite preview",
  "tauri": "tauri"
}
```

## Process

1. Read `package.json` to verify current scripts
2. Understand the request in context of Tauri + Vite + React project
3. Add or modify scripts as needed

## Common Operations

```bash
npm run dev          # Vite dev server (frontend only, port 1420)
npm run build        # TypeScript check + Vite production build
npm run preview      # Preview production build
npm run tauri dev    # Full Tauri dev (frontend + Rust backend)
npm run tauri build  # Production Tauri bundle
```

## Project Dependencies

- **Runtime**: react, react-dom, @tauri-apps/api, @tauri-apps/plugin-opener
- **Dev**: @vitejs/plugin-react, typescript, vite, @tauri-apps/cli
- **Not installed**: eslint, prettier, vitest, jest (add as needed)

## Notes

- This is a Tauri v2 project — most dev/build workflows go through `tauri` CLI
- Vite config: `vite.config.ts` (React plugin, HMR port 1421)
- TypeScript config: `tsconfig.json` (strict, ES2020, react-jsx)
