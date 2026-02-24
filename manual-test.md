# LazyRec v0.3.0 — Manual Test Checklist

Run the Windows installer (`LazyRec_0.3.0_x64-setup.exe` or `.msi`) from the GitHub release, then work through each section.

---

## 1. Installation & Launch

- [x] Installer runs without errors
- [x] App launches and shows the Welcome screen
- [x] Window title says "LazyRec"
- [x] App icon appears in the taskbar

## 2. Welcome Screen

- [x] Logo, title "LazyRec", and subtitle "Screen Recording & Timeline Editing" are visible
- [x] Three action cards displayed: **Record**, **Open Video**, **Open Project**
- [x] Hovering a card lifts it slightly (translateY animation)
- [x] Drop zone ("Drop video or project here") is visible below cards
- [ ] Dragging a file over the drop zone highlights the border blue
- [ ] Dropping a `.mp4` file transitions to the Editor screen
- [x] Clicking **Record** transitions to the Recording screen
- [x] Clicking **Open Video** transitions to the Editor screen
- [x] Clicking **Open Project** transitions to the Editor screen

## 3. Recording Screen

- [x] "Recording" header and back button visible
- [x] Back button returns to the Welcome screen
- [x] Source selector dropdown populates with real display/window names on load
- [x] Selecting a different source updates the displayed name and dimensions
- [ ] Clicking **Start Recording** calls `set_capture_target` before starting (invalid args `target` for command `set_capture_target`: missing field `display_id`)
- [x] 3-second countdown appears (3 → 2 → 1) with pulse animation
- [ ] After countdown, state changes to "REC" with blinking red dot
- [ ] Timer counts up in `MM:SS` format
- [ ] **Pause** button changes label to "PAUSED", dot stops blinking, timer freezes
- [ ] **Resume** button resumes timer from where it paused
- [ ] **Stop** button transitions to the Editor screen
- [ ] Elapsed time is consistent after multiple pause/resume cycles

## 4. Editor Screen — Layout

- [ ] Header shows: back button, "Timeline Editor" title, **Generate** button, transport controls, export button
- [ ] Main area is split: video preview (left) and inspector panel (right)
- [ ] Timeline panel at bottom with ruler, zoom controls, and 4 tracks
- [ ] Overall layout fills the window without scrollbars on the main area

## 5. Editor Screen — Transport Controls

- [ ] **Rewind** button (⏮) resets playhead to 00:00
- [ ] **Play** button (▶) starts playback, changes to ⏸
- [ ] **Pause** button (⏸) stops playback, changes to ▶
- [ ] Time display shows `MM:SS / MM:SS` (current / duration)
- [ ] Playhead resets to 00:00 and stops when reaching the end

## 6. Editor Screen — Video Preview

- [ ] Preview area shows a dark canvas with gradient background
- [ ] Dashed blue viewport indicator visible, animates with playhead
- [ ] White cursor dot visible, moves smoothly with playhead
- [ ] Viewport shrinks as playhead advances (simulated zoom-in)
- [ ] Timecode overlay in the bottom-right of the preview
- [ ] When a project is loaded, preview uses the actual project video frames

## 7. Editor Screen — Timeline

- [ ] Ruler shows time markers (00:00, 00:05, 00:10, etc.)
- [ ] Clicking the ruler seeks the playhead to that time
- [ ] Red playhead line visible in ruler and tracks
- [ ] 4 tracks visible: Transform (blue), Ripple (red), Cursor (orange), Keystroke (green)
- [ ] Track labels have colored left borders matching their type
- [ ] Diamond-shaped keyframe markers at correct time positions
- [ ] Hovering a keyframe marker scales it up
- [ ] Clicking a keyframe marker selects it (white outline glow)
- [ ] Clicking a keyframe navigates playhead to that keyframe's time
- [ ] Playhead line in tracks moves during playback

### 7a. Timeline — Keyframe Dragging

- [ ] Clicking and dragging a keyframe marker moves it along the time axis
- [ ] Cursor changes to grab/grabbing during drag
- [ ] Keyframe snaps to 10ms intervals while dragging
- [ ] Dragging clamps to timeline bounds (0 to duration)
- [ ] Releasing the mouse finalizes the keyframe's new position

### 7b. Timeline — Zoom & Scale

- [ ] Zoom slider visible in the timeline zoom bar
- [ ] Dragging the zoom slider scales the timeline width (1x–20x)
- [ ] Ctrl+scroll wheel zooms in/out on the timeline
- [ ] Zoom label shows the current zoom level (e.g., "1.0x")
- [ ] Reset button ("1x") resets zoom to default
- [ ] Horizontal scrollbar appears when zoomed beyond viewport width
- [ ] Keyframe positions remain accurate at all zoom levels

## 8. Editor Screen — Generate Keyframes

- [ ] Green **Generate** button visible in the toolbar
- [ ] Clicking Generate calls the backend `generate_keyframes` command
- [ ] Timeline tracks refresh with auto-generated keyframes after generation
- [ ] Duration updates from the backend if a project is loaded

## 9. Editor Screen — Undo/Redo

- [ ] Ctrl+Z undoes the last timeline change (keyframe move, generation, etc.)
- [ ] Ctrl+Shift+Z or Ctrl+Y redoes the undone change
- [ ] Undo stack holds up to 50 snapshots
- [ ] Loading from backend (via Generate or project load) clears the undo stack
- [ ] Undo/redo does not affect raw backend loads

## 10. Editor Screen — Inspector Panel (Properties Tab)

- [ ] When no keyframe selected: shows diamond icon + "Select a keyframe to inspect"
- [ ] After selecting a **Transform** keyframe: shows Zoom, Center X, Center Y, Easing
- [ ] After selecting a **Ripple** keyframe: shows Intensity, Duration, Color
- [ ] After selecting a **Keystroke** keyframe: shows Text, Duration
- [ ] After selecting a **Cursor** keyframe: shows "No editable properties"
- [ ] Badge shows track type with matching color
- [ ] Time shown as `@ MM:SS`
- [ ] Easing presets row (linear, easeIn, easeOut, easeInOut, spring)
- [ ] Active easing button highlighted in blue

## 11. Editor Screen — Inspector Panel (Settings Tab)

- [ ] Clicking "Settings" tab switches to render settings
- [ ] **Output** section: Resolution, Codec, Quality, Frame Rate dropdowns visible
- [ ] Settings load from backend on mount (not hardcoded defaults)
- [ ] Changing any setting saves to backend immediately
- [ ] Resolution options: Original, 4K, 1440p, 1080p, 720p
- [ ] Codec options: H.265 (HEVC), H.264
- [ ] Quality options: High, Medium, Low, Original
- [ ] Frame Rate options: Original, 60 fps, 30 fps
- [ ] **Window Mode** section: Background checkbox, Corner Radius, Shadow fields
- [ ] Corner Radius slider/input updates value (0–50)
- [ ] Shadow Opacity slider/input updates value (0–100)
- [ ] Switching back to "Properties" tab restores previous selection

## 12. Editor Screen — Export (Async with Progress)

- [ ] **Export** button visible in the transport bar
- [ ] Clicking Export starts the export process asynchronously
- [ ] Button text changes to "Exporting..." and becomes disabled
- [ ] Progress bar appears below the header with gradient fill (blue → green)
- [ ] Progress text shows frame count, percentage, and ETA
- [ ] Progress updates in real time via Tauri event stream
- [ ] On completion, "Export complete" message and path displayed
- [ ] On failure, "Export failed" message with error displayed
- [ ] After completion/failure, button re-enables
- [ ] Check `Videos/LazyRec/export.mp4` was created on disk

## 13. Window Behavior

- [ ] Window is resizable
- [ ] Layout adapts to smaller window sizes (no overlapping elements)
- [ ] Closing the window exits the app cleanly (no zombie processes)

## 14. Dark Theme & Visuals

- [ ] All screens use dark background (#0f0f23)
- [ ] Text is readable (light on dark)
- [ ] Buttons have hover states (color changes, slight transforms)
- [ ] No visible style glitches or unstyled elements
- [ ] Fonts render correctly (system sans-serif)

---

## Notes

- The recording backend uses real Windows capture/input hooks via `windows-capture` and `SetWindowsHookEx`. Capture sources enumerate real displays and windows. Actual frame recording requires the full pipeline (encoder writes stub output without FFmpeg).
- Export uses a stub video source (gradient test frames) and stub encoder unless built with `--features ffmpeg`. The async progress pipeline and event streaming are real.
- Inspector fields are read-only for now (display only, no editing).
- Timeline tracks load from the Rust backend via `get_timeline` — no longer hardcoded seed data.
