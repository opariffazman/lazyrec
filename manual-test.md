# LazyRec v0.2.0 — Manual Test Checklist

Run the Windows installer (`LazyRec_0.2.0_x64-setup.exe` or `.msi`) from the GitHub release, then work through each section.

---

## 1. Installation & Launch

- [ ] Installer runs without errors
- [ ] App launches and shows the Welcome screen
- [ ] Window title says "LazyRec"
- [ ] App icon appears in the taskbar

## 2. Welcome Screen

- [ ] Logo, title "LazyRec", and subtitle "Screen Recording & Timeline Editing" are visible
- [ ] Three action cards displayed: **Record**, **Open Video**, **Open Project**
- [ ] Hovering a card lifts it slightly (translateY animation)
- [ ] Drop zone ("Drop video or project here") is visible below cards
- [ ] Dragging a file over the drop zone highlights the border blue
- [ ] Dropping a `.mp4` file transitions to the Editor screen
- [ ] Clicking **Record** transitions to the Recording screen
- [ ] Clicking **Open Video** transitions to the Editor screen
- [ ] Clicking **Open Project** transitions to the Editor screen

## 3. Recording Screen

- [ ] "Recording" header and back button visible
- [ ] Back button returns to the Welcome screen
- [ ] Source selector dropdown shows "Entire Screen" and "Window..."
- [ ] Clicking **Start Recording** shows a 3-second countdown (3 → 2 → 1)
- [ ] Countdown numbers pulse with animation
- [ ] After countdown, state changes to "REC" with blinking red dot
- [ ] Timer counts up in `MM:SS` format
- [ ] **Pause** button changes label to "PAUSED", dot stops blinking, timer freezes
- [ ] **Resume** button resumes timer from where it paused
- [ ] **Stop** button transitions to the Editor screen
- [ ] Elapsed time is consistent after multiple pause/resume cycles

## 4. Editor Screen — Layout

- [ ] Header shows: back button, "Timeline Editor" title, transport controls, export button
- [ ] Main area is split: video preview (left) and inspector panel (right)
- [ ] Timeline panel at bottom with ruler and 4 tracks
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

## 8. Editor Screen — Inspector Panel (Properties Tab)

- [ ] When no keyframe selected: shows diamond icon + "Select a keyframe to inspect"
- [ ] After selecting a **Transform** keyframe: shows Zoom, Center X, Center Y, Easing
- [ ] After selecting a **Ripple** keyframe: shows Intensity, Duration, Color
- [ ] After selecting a **Keystroke** keyframe: shows Text, Duration
- [ ] After selecting a **Cursor** keyframe: shows "No editable properties"
- [ ] Badge shows track type with matching color
- [ ] Time shown as `@ MM:SS`
- [ ] Easing presets row (linear, easeIn, easeOut, easeInOut, spring)
- [ ] Active easing button highlighted in blue

## 9. Editor Screen — Inspector Panel (Settings Tab)

- [ ] Clicking "Settings" tab switches to render settings
- [ ] **Output** section: Resolution, Codec, Quality, Frame Rate dropdowns visible
- [ ] Resolution options: Original, 4K, 1440p, 1080p, 720p
- [ ] Codec options: H.265 (HEVC), H.264
- [ ] Quality options: High, Medium, Low, Original
- [ ] Frame Rate options: Original, 60 fps, 30 fps
- [ ] **Window Mode** section: Background checkbox, Corner Radius, Shadow fields
- [ ] Switching back to "Properties" tab restores previous selection

## 10. Editor Screen — Export

- [ ] **Export** button visible in the transport bar
- [ ] Clicking Export starts the export process
- [ ] Button text changes to "Exporting..." and becomes disabled
- [ ] Progress bar appears below the header with gradient fill (blue → green)
- [ ] Progress text shows percentage or "Export complete" / "Export failed"
- [ ] After completion, button re-enables
- [ ] Check `Videos/LazyRec/export.mp4` was created on disk

## 11. Window Behavior

- [ ] Window is resizable
- [ ] Layout adapts to smaller window sizes (no overlapping elements)
- [ ] Closing the window exits the app cleanly (no zombie processes)

## 12. Dark Theme & Visuals

- [ ] All screens use dark background (#0f0f23)
- [ ] Text is readable (light on dark)
- [ ] Buttons have hover states (color changes, slight transforms)
- [ ] No visible style glitches or unstyled elements
- [ ] Fonts render correctly (system sans-serif)

---

## Notes

- The recording backend uses platform stubs on this build — actual screen capture is not yet wired. Recording flow tests the UI state machine only.
- Export uses a stub video source (generates gradient test frames) and stub encoder (no actual video output). The pipeline orchestration is real.
- Inspector fields are read-only for now (display only, no editing).
