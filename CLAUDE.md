# CLAUDE.md — LazyRec (Screenize Cross-Platform Replication)

This file provides guidance to Claude Code when working on LazyRec, a cross-platform screen recording application inspired by [Screenize](https://github.com/syi0808/Screenize).

## Project Goal

Replicate Screenize's functionality as a **cross-platform** desktop app (Windows + Linux, optionally macOS). Screenize is a macOS-only Swift app that captures screen recordings with mouse/keyboard tracking and applies post-production effects (auto-zoom, click effects, cursor styling, keystroke overlays) via a timeline editor.

## Reference Implementation

The original Screenize repo is cloned at `_reference/` for study. **Do not modify files in `_reference/`.**

- **License**: Apache 2.0
- **Author**: Sung YeIn ([GitHub](https://github.com/syi0808))
- **Latest version**: v0.3.1 (Feb 2026), 46 commits, 225 stars

---

## Screenize Architecture Deep Dive

### Three-Phase Processing Model

```
Phase 1: RECORDING
  Screen capture → raw video file (MP4/MOV)
  Input tracking  → mouse.json (positions, clicks, keyboard, scrolls, drags, UI state)

Phase 2: EDITING
  Load recording + mouse data into timeline editor
  Auto-generate keyframes from mouse data (smart zoom, click effects, keystrokes)
  User manually adjusts keyframes on multi-track timeline

Phase 3: EXPORT
  Read raw video frame-by-frame
  Evaluate timeline state per frame (interpolate keyframes with easing)
  Render effects (zoom/pan, ripple, cursor, keystroke overlay)
  Encode to output MP4/MOV
```

### Source Structure (`_reference/Screenize/`)

```
Screenize/
├── App/                    # AppState (singleton), GlobalHotkeyManager, SparkleController
├── Core/
│   ├── Capture/            # ScreenCaptureKit wrapper, permissions
│   ├── Coordinates.swift   # Three coordinate spaces + converter
│   ├── EventMonitoring/    # NSEvent global/local monitor manager
│   ├── Recording/          # VideoWriter, RecordingCoordinator, CFRRecordingManager
│   │   └── EventHandlers/  # Click, Drag, Keyboard, Scroll handlers
│   └── Tracking/           # MouseTracker, ClickDetector, AccessibilityInspector, ZoomCalculator
├── Generators/             # Auto-keyframe generation from mouse data
│   └── SmartZoom/          # Session clustering + zoom level calculation
├── Models/                 # Data models (BackgroundStyle, CaptureTarget, ClickType, etc.)
├── Project/                # .screenize package format, project CRUD, presets
├── Render/                 # ExportEngine, FrameEvaluator, Renderer, PreviewEngine
├── Timeline/               # Timeline, Track, Keyframe, EasingCurve
├── ViewModels/             # EditorViewModel, PermissionWizardViewModel, UndoStack
└── Views/                  # SwiftUI views (Editor, Recording, Timeline, Inspector, etc.)
```

### Coordinate Systems

Three coordinate spaces, all conversions centralized in `CoordinateConverter`:

| Space | Origin | Range | Used For |
|-------|--------|-------|----------|
| ScreenPoint | Bottom-left (macOS) | Pixel units | NSEvent.mouseLocation |
| CapturePixelPoint | Bottom-left | Pixel units | Stored in mouse.json |
| NormalizedPoint | Bottom-left | 0.0–1.0 | **Internal standard** — keyframes, timeline, rendering |

**Cross-platform note**: Windows/Linux use top-left origin. Our implementation should standardize on top-left origin for NormalizedPoint and adapt at platform boundaries.

### Recording Pipeline

```
RecordingCoordinator (orchestrator)
├── ScreenCaptureManager (macOS ScreenCaptureKit)
│   ├── SCStream → video frame callbacks (VFR)
│   └── CFRRecordingManager → converts VFR to 60fps CFR
│       └── VideoWriter (AVAssetWriter) → MP4 file
└── MouseDataRecorder
    ├── 60Hz position sampler (NSEvent.mouseLocation)
    ├── 1Hz UI state sampler (AccessibilityInspector / AXUIElement)
    ├── ClickEventHandler (NSEvent mouseDown/Up)
    ├── ScrollEventHandler (NSEvent scrollWheel)
    ├── KeyboardEventHandler (CGEventTap — global keyboard hook)
    └── DragEventHandler (NSEvent mouseDragged)
```

**Output**: video file + `<video>.mouse.json` with all event data

### Mouse Data Format (v4 / polyrecorder-compatible)

```
metadata.json          # Recording metadata
mousemoves-0.json      # {timestamp, x, y} at 60Hz
mouseclicks-0.json     # {timestamp, x, y, type, duration}
keystrokes-0.json      # {timestamp, keyCode, character, modifiers, type}
```

Timestamps use both `processTimeMs` (system uptime) and `unixTimeMs` (wall clock), relative to recording start.

### Timeline & Keyframe System

```
Timeline
├── TransformTrack     → [TransformKeyframe {time, zoom, center, easing}]
├── RippleTrack        → [RippleKeyframe {time, position, intensity, duration}]
├── CursorTrack        → [CursorStyleKeyframe {time, style, scale, visible}]
└── KeystrokeTrack     → [KeystrokeKeyframe {time, text, position, duration}]
```

- Keyframes always sorted by time within tracks
- `AnyTrack` type-erased wrapper for Codable serialization
- Track protocol defines common interface
- EasingCurves: linear, easeIn, easeOut, easeInOut, cubicBezier, spring (critically damped)

### Keyframe Generators

Auto-generate keyframes from mouse data:

| Generator | Input | Output |
|-----------|-------|--------|
| SmartZoomGenerator | positions, clicks, drags, keyboard | TransformKeyframes (zoom/pan) |
| RippleGenerator | clicks | RippleKeyframes |
| CursorInterpolationGenerator | positions | Smoothed cursor path |
| ClickCursorGenerator | clicks | CursorStyleKeyframes |
| KeystrokeGenerator | keyboard events | KeystrokeKeyframes |

**SmartZoom Pipeline**:
```
ActivityCollector (clicks + drags + typing sessions)
  → SessionClusterer (group by time 3s / distance 0.3 normalized)
    → ZoomLevelCalculator (bounding box → zoom level, target 70% coverage)
      → SessionCenterResolver (cursor position + saliency analysis)
        → Generate zoom-in → hold → zoom-out keyframes
```

### Render Pipeline

```
ExportEngine (orchestrator)
├── AVAssetReader → extract video frames
├── MouseDataConverter → convert mouse.json to render format
├── FrameEvaluator → evaluate timeline state at time t
│   ├── Binary search for bounding keyframes
│   ├── Easing curve interpolation
│   └── Velocity computation (easing derivatives → motion blur)
├── Renderer → composite effects onto frame
│   ├── Ripple: CIRadialGradient overlay
│   ├── Cursor: NSCursor image extraction + positioning
│   ├── Transform: crop/zoom/pan (CoreImage transforms)
│   ├── Keystroke: NSAttributedString → pill overlay
│   └── Motion blur: CIMotionBlur filter
└── AVAssetWriter → encode output MP4/ProRes
```

**Composition order**: Ripple → Cursor → Transform (crop/zoom) → Keystroke overlay → Motion blur

### Project Format

```
MyProject.screenize/           # macOS package directory
├── project.json               # ScreenizeProject v2 (timeline + settings + metadata)
└── recording/
    ├── recording.mp4          # Raw captured video
    └── recording.mouse.json   # Mouse/keyboard tracking data
```

---

## macOS-Specific APIs → Cross-Platform Alternatives

### Screen Capture

| macOS | Windows | Linux |
|-------|---------|-------|
| ScreenCaptureKit (macOS 15+) | DXGI Desktop Duplication, WinRT GraphicsCapture | PipeWire, X11 (XShm/XComposite), Wayland (wlr-screencopy) |
| SCStream (frame callbacks) | IDXGIOutputDuplication::AcquireNextFrame | PipeWire stream, X11 XShmGetImage |
| SCShareableContent (enumerate sources) | EnumWindows, MonitorFromWindow | X11 XQueryTree, wlr-foreign-toplevel |

### Video Encoding/Decoding

| macOS | Cross-Platform |
|-------|----------------|
| AVAssetWriter | FFmpeg (libavcodec/libavformat) |
| AVAssetReader | FFmpeg (libavcodec/libavformat) |
| CVPixelBuffer | Raw byte buffers, FFmpeg AVFrame |
| CoreMedia CMTime | Custom rational time type or FFmpeg AVRational |

### Graphics / Image Processing

| macOS | Cross-Platform |
|-------|----------------|
| CoreImage (CIImage, CIFilter, CIContext) | Skia, OpenGL/Vulkan shaders, wgpu |
| CIRadialGradient | Fragment shader |
| CIMotionBlur | Blur shader kernel |
| NSGraphicsContext | Skia Canvas, Cairo |
| NSCursor (cursor images) | Load cursor PNGs/SVGs, Xcursor (Linux), GetCursorInfo (Windows) |

### Input Monitoring

| macOS | Windows | Linux |
|-------|---------|-------|
| NSEvent global monitors | SetWindowsHookEx (WH_MOUSE_LL, WH_KEYBOARD_LL) | X11: XGrabKey/XGrabPointer, libinput |
| CGEventTap (keyboard) | Raw Input API, SetWindowsHookEx | X11 XRecord extension, libinput |
| NSEvent.mouseLocation | GetCursorPos | X11 XQueryPointer, libinput |
| NSEvent scrollWheel | WM_MOUSEWHEEL | X11 ButtonPress (4/5), libinput scroll |

### Accessibility / UI Inspection

| macOS | Windows | Linux |
|-------|---------|-------|
| AXUIElement | UI Automation (IUIAutomation) | AT-SPI2 (via libatspi) |
| kAXRoleAttribute | UIA_ControlTypePropertyId | AT-SPI2 role enum |
| AXUIElementCopyElementAtPosition | IUIAutomation::ElementFromPoint | AT-SPI2 GetAccessibleAtPoint |

### Permissions

| macOS | Windows | Linux |
|-------|---------|-------|
| CGPreflightScreenCaptureAccess | Manifest capabilities, admin | PipeWire portal permissions |
| AXIsProcessTrustedWithOptions | No equivalent (always available) | AT-SPI2 enabled check |
| CGRequestListenEventAccess | No equivalent (low-level hooks need elevation) | User in `input` group |
| AVCaptureDevice.authorizationStatus(.audio) | MediaCapture capability | PulseAudio/PipeWire permissions |

---

## Recommended Tech Stack for Cross-Platform Build

### Option A: Rust + Tauri (Recommended)

```
UI Layer:        Tauri v2 + TypeScript/React (web-based UI)
Core Logic:      Rust (timeline, keyframes, easing, coordinate math)
Screen Capture:  Platform-specific Rust crates or FFI:
                   - Windows: windows-capture crate / DXGI bindings
                   - Linux: pipewire-rs / x11rb
Video I/O:       FFmpeg via ffmpeg-next (Rust bindings)
Rendering:       wgpu (WebGPU) or Skia (via skia-safe)
Input Hooks:     Platform-specific:
                   - Windows: windows crate (SetWindowsHookEx)
                   - Linux: x11rb + evdev/libinput
Accessibility:   Platform-specific:
                   - Windows: uiautomation crate
                   - Linux: atspi crate
```

### Option B: Electron + Node.js

```
UI Layer:        Electron + React
Core Logic:      TypeScript (timeline, keyframes, easing)
Screen Capture:  Electron desktopCapturer (limited) or native addons
Video I/O:       FFmpeg via fluent-ffmpeg or native addon
Rendering:       Canvas/WebGL in renderer process, or Sharp/FFmpeg for export
Input Hooks:     Native Node addons (node-global-key-listener, iohook)
Accessibility:   Native Node addons per platform
```

### Option C: C++ / Qt

```
UI Layer:        Qt 6 (QML or Widgets)
Core Logic:      C++ (timeline, keyframes, easing)
Screen Capture:  Platform-specific Qt screen capture or direct API
Video I/O:       FFmpeg (C API)
Rendering:       Qt Quick Scene Graph or OpenGL
Input Hooks:     Platform-specific (direct Win32/X11 APIs)
Accessibility:   Qt Accessibility bridge
```

---

## What's Platform-Agnostic (Port Directly)

These components from Screenize contain **zero platform-specific code** and can be ported 1:1:

1. **Timeline/Keyframe model** — Track, Keyframe, Timeline data structures
2. **Easing curves** — linear, easeIn/Out, cubicBezier, spring (pure math)
3. **Keyframe interpolation** — binary search + easing application
4. **Coordinate math** — NormalizedPoint operations, viewport calculations
5. **Smart zoom algorithms** — activity collection, session clustering, zoom level calculation
6. **Mouse data format** — JSON schema for positions/clicks/keyboard/scrolls/drags
7. **Project format** — JSON-based project structure
8. **Transform math** — crop rect calculation from zoom/center
9. **Render settings** — codec, quality, resolution configuration
10. **Undo/redo stack** — generic state management

**Estimated split: ~60% platform-agnostic, ~40% platform-specific**

---

## Implementation Priority

### Phase 1: Core Recording (MVP)
1. Screen/window capture with source selection
2. Raw video encoding to MP4 (via FFmpeg)
3. Mouse position tracking at 60Hz
4. Click event recording
5. Keyboard event recording
6. Mouse data JSON export (compatible format)
7. Basic project file format

### Phase 2: Timeline Editor
1. Timeline data model (tracks, keyframes)
2. Basic timeline UI (playhead, tracks, keyframe markers)
3. Video preview with frame extraction
4. Manual keyframe creation/editing
5. Easing curve support

### Phase 3: Auto-Generation
1. Smart zoom generator (activity → sessions → zoom keyframes)
2. Ripple generator (click → ripple keyframes)
3. Keystroke generator (keyboard → keystroke keyframes)
4. Cursor interpolation generator

### Phase 4: Render & Export
1. Frame-by-frame render pipeline
2. Transform application (crop/zoom/pan)
3. Ripple effect rendering
4. Cursor rendering
5. Keystroke overlay rendering
6. Motion blur (optional)
7. Export to MP4 with progress tracking

### Phase 5: Polish
1. Background styling (solid, gradient, image)
2. Custom cursor styles
3. Window mode rendering (shadow, rounded corners)
4. Preset system
5. Accessibility-driven zoom targeting (UI element detection)

---

## Key Design Decisions to Make

1. **Language/Framework**: Rust+Tauri vs Electron vs Qt (see options above)
2. **Coordinate origin**: Standardize on top-left origin (unlike Screenize's bottom-left)
3. **Video backend**: FFmpeg is the obvious choice for cross-platform
4. **GPU rendering**: wgpu/WebGPU vs OpenGL vs software rendering
5. **Input hooking strategy**: Per-platform native or abstraction library
6. **Project format**: Keep Screenize's `.screenize` package format or design new one
7. **Mouse data format**: Keep polyrecorder v4 compatibility or simplify

---

## Reference Files (Key Reading)

| File | What to Learn |
|------|---------------|
| `_reference/CLAUDE.md` | Full architecture doc with conventions |
| `_reference/Screenize/Core/Coordinates.swift` | Coordinate system design (426 lines) |
| `_reference/Screenize/Core/Recording/RecordingCoordinator.swift` | Recording orchestration pattern |
| `_reference/Screenize/Core/Recording/MouseDataRecorder.swift` | Input data collection design |
| `_reference/Screenize/Core/Recording/EventStreamWriter.swift` | Mouse data JSON format |
| `_reference/Screenize/Timeline/Timeline.swift` | Timeline data model |
| `_reference/Screenize/Timeline/Track.swift` | Track/keyframe protocols |
| `_reference/Screenize/Timeline/EasingCurve.swift` | Easing math |
| `_reference/Screenize/Render/FrameEvaluator.swift` | Keyframe interpolation |
| `_reference/Screenize/Render/Renderer.swift` | Effect composition |
| `_reference/Screenize/Render/ExportEngine.swift` | Export pipeline |
| `_reference/Screenize/Generators/SmartZoomGenerator.swift` | Smart zoom algorithm |
| `_reference/Screenize/Generators/SmartZoom/*.swift` | Zoom sub-algorithms |
| `_reference/Screenize/Models/EventStream.swift` | Mouse data model |
| `_reference/Screenize/Project/ScreenizeProject.swift` | Project format |

---

## Conventions

- All code, comments, and documentation in English
- Commit messages use imperative mood ("Add feature" not "Added feature")
- Normalized coordinates use 0–1 range for all internal position data
- Keyframes always sorted by time within tracks
- Platform-specific code isolated behind trait/interface boundaries
