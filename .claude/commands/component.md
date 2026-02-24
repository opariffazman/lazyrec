# React Component Generator

Create a React component named $ARGUMENTS for LazyRec.

## Steps

1. **Check existing components**: Read `src/App.tsx` to understand current patterns (functional components, hooks, plain CSS)
2. **Determine placement**: Components live in `src/` — check if a `src/components/` directory should be created or if it belongs in App.tsx
3. **Implement component**: Write TypeScript with proper props interface
4. **Style it**: Use plain CSS in `src/App.css` (no CSS-in-JS — project convention)
5. **Wire Tauri IPC if needed**: Import from `@tauri-apps/api` for backend communication

## Project Conventions

- React 19 with hooks (useState, useRef, useCallback, useEffect)
- TypeScript strict mode (noUnusedLocals, noUnusedParameters)
- Plain CSS styling in App.css
- Tauri v2 IPC via `@tauri-apps/api/core` invoke()
- No external UI libraries — custom components only
- Three existing screens: WelcomeScreen, RecordingScreen, EditorScreen
