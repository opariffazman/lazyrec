# React Hooks

Create or optimize React hooks for $ARGUMENTS in LazyRec.

## Process

1. **Check existing code**: Read `src/App.tsx` to understand current hook usage patterns
2. **Identify hook type**: New custom hook, optimization, or Tauri IPC hook
3. **Implement**: Write with proper TypeScript types and cleanup
4. **Verify**: Run `npx tsc --noEmit` to check types

## Project Context

- React 19 with hooks (useState, useRef, useCallback, useEffect)
- TypeScript strict mode
- Tauri v2 IPC: `invoke()` from `@tauri-apps/api/core`
- No state management library — hooks + local state only
- Three screens sharing state via prop drilling in App.tsx

## Common Hook Patterns for This Project

- **useTauriCommand**: Wrap `invoke()` with loading/error state
- **useRecording**: Recording state machine (idle → countdown → recording → paused)
- **useTimeline**: Timeline playback, playhead position, track management
- **useKeyframes**: Keyframe CRUD operations on tracks
- **useHotkeys**: Global keyboard shortcuts for recording/editing
- **useVideoPreview**: Frame extraction and preview rendering

## Requirements

- Follow existing patterns in App.tsx
- Include cleanup in useEffect (timers, listeners)
- Type all parameters and return values
- Keep hooks in `src/hooks/` if extracting from App.tsx
