# React State Management

Implement state management for $ARGUMENTS in LazyRec.

## Current Approach

LazyRec uses React hooks + local state in App.tsx. No external state library.

## Process

1. **Analyze current state**: Read `src/App.tsx` to understand existing state shape
2. **Determine scope**: Local component state vs shared app state
3. **Implement**: Use appropriate pattern for the complexity

## State Architecture

### Current State in App.tsx
- `currentScreen`: 'welcome' | 'recording' | 'editor'
- Recording state: status, timer, countdown, capture settings
- Editor state: timeline tracks, playhead position, playing flag, keyframes

### Recommended Patterns (by complexity)
- **Component state**: `useState` + `useReducer` (current approach)
- **Shared state**: React Context (when components extracted from App.tsx)
- **Complex state**: Zustand (lightweight, no boilerplate — add only if needed)

### Tauri Backend State
- Project data lives in Rust (timeline, tracks, keyframes, project metadata)
- Frontend syncs via Tauri IPC `invoke()` commands
- Backend is source of truth for all persistent data

## Important Notes

- Don't add state management libraries unless complexity demands it
- Prefer Tauri IPC for data that should persist
- Keep UI-only state (hover, focus, animation) in React
