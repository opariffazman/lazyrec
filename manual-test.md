# LazyRec v0.3.2 — Manual Test Checklist

Run the Windows installer (`LazyRec_0.3.1_x64-setup.exe` or `.msi`) from the GitHub release, then work through each section.

---

## 1. Installation & Launch

- [ ] Installer runs without errors
- [ ] App launches and shows the Welcome screen
- [ ] Window title says "LazyRec"
- [ ] App icon appears in the taskbar
- [ ] FFmpeg DLLs are present alongside the executable (avcodec-61.dll, avformat-61.dll, etc.)

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
- [ ] Source selector dropdown populates with real display/window names on load
- [ ] Selecting a different source updates the displayed name and dimensions
- [ ] Clicking **Start Recording** sets capture target with correct dimensions
- [ ] 3-second countdown appears (3 → 2 → 1) with pulse animation
- [ ] After countdown, state changes to "REC" with blinking red dot
- [ ] Timer counts up in `MM:SS` format
- [ ] **Pause** button changes label to "PAUSED", dot stops blinking, timer freezes
- [ ] **Resume** button resumes timer from where it paused
- [ ] **Stop** button transitions to the Post-Recording screen
- [ ] Elapsed time is consistent after multiple pause/resume cycles

## 4. Post-Recording Screen (NEW in v0.3.2)

- [ ] "Recording Complete" header and back button visible
- [ ] Two choice cards displayed: **Export with Auto-Zoom** (highlighted) and **Open in Editor**
- [ ] Hovering cards shows lift animation
- [ ] Clicking **Open in Editor** transitions to the Editor screen

### 4a. Quick Export Flow (Export with Auto-Zoom)

- [ ] Clicking **Export with Auto-Zoom** shows spinner with "Generating auto-zoom keyframes..."
- [ ] After generation, message updates with keyframe count
- [ ] Progress bar appears during export with percentage and ETA
- [ ] On completion, green checkmark with "Export Complete" and file path
- [ ] **Done** button returns to the Welcome screen
- [ ] Exported MP4 file exists at the displayed path and is playable
- [ ] Exported video shows zoom effects at click/activity locations

### 4b. Quick Export Error Handling

- [ ] If export fails, red X with error message displayed
- [ ] **Try Again** button resets to the choice screen
- [ ] **Open Editor Instead** button transitions to the Editor screen

## 5. Recording Output Verification

- [ ] After recording, check `Videos/LazyRec/` for:
  - [ ] `recording_<timestamp>.mp4` — playable video of the captured screen
  - [ ] `recording_<timestamp>_mouse.json` — mouse/keyboard tracking data
  - [ ] `Recording_<timestamp>.lazyrec/` — project package directory
- [ ] The recorded MP4 plays correctly in VLC or Windows Media Player
- [ ] The MP4 has correct dimensions matching the captured source
- [ ] The MP4 has audio/video sync (if recording lasted >5 seconds)

## 6. Editor Screen — Layout

- [ ] Header shows: back button, "Timeline Editor" title, **Generate** button, transport controls, export button
- [ ] Main area is split: video preview (left) and inspector panel (right)
- [ ] Timeline panel at bottom with ruler, zoom controls, and 4 tracks
- [ ] Overall layout fills the window without scrollbars on the main area

## 7. Editor Screen — Transport Controls

- [ ] **Rewind** button (⏮) resets playhead to 00:00
- [ ] **Play** button (▶) starts playback, changes to ⏸
- [ ] **Pause** button (⏸) stops playback, changes to ▶
- [ ] Time display shows `MM:SS / MM:SS` (current / duration)
- [ ] Playhead resets to 00:00 and stops when reaching the end

## 8. Editor Screen — Video Preview

- [ ] Preview area shows a dark canvas with gradient background
- [ ] Dashed blue viewport indicator visible, animates with playhead
- [ ] White cursor dot visible, moves smoothly with playhead
- [ ] Viewport shrinks as playhead advances (simulated zoom-in)
- [ ] Timecode overlay in the bottom-right of the preview
- [ ] When a project is loaded, preview uses the actual project video frames

## 9. Editor Screen — Timeline

- [ ] Ruler shows time markers (00:00, 00:05, 00:10, etc.)
- [ ] Clicking the ruler seeks the playhead to that time
- [ ] Red playhead line visible in ruler and tracks
- [ ] 4 tracks visible: Transform (blue), Ripple (red), Cursor (orange), Keystroke (green)
- [ ] Diamond-shaped keyframe markers at correct time positions
- [ ] Clicking a keyframe marker selects it (white outline glow)
- [ ] Keyframe dragging works along the time axis

## 10. Editor Screen — Generate Keyframes

- [ ] Green **Generate** button visible in the toolbar
- [ ] Clicking Generate calls the backend `generate_keyframes` command
- [ ] Timeline tracks refresh with auto-generated keyframes after generation

## 11. Editor Screen — Export

- [ ] **Export** button visible in the transport bar
- [ ] Clicking Export starts the export process asynchronously
- [ ] Button text changes to "Exporting..." and becomes disabled
- [ ] Progress bar appears with gradient fill (blue → green)
- [ ] Progress text shows frame count, percentage, and ETA
- [ ] On completion, "Export complete" message displayed
- [ ] Check `Videos/LazyRec/export_<timestamp>.mp4` was created on disk
- [ ] Exported video contains zoom/pan effects from the generated keyframes

## 12. Window Behavior

- [ ] Window is resizable
- [ ] Layout adapts to smaller window sizes (no overlapping elements)
- [ ] Closing the window exits the app cleanly (no zombie processes)

## 13. Dark Theme & Visuals

- [ ] All screens use dark background (#0f0f23)
- [ ] Text is readable (light on dark)
- [ ] Buttons have hover states (color changes, slight transforms)
- [ ] No visible style glitches or unstyled elements

---

## Notes

- **v0.3.2 changes**: FFmpeg is now enabled by default. Recording produces real MP4 files (not stubs). Export applies actual zoom/pan effects. FFmpeg DLLs are bundled with the installer.
- The recording backend uses real Windows capture via `windows-capture` and input hooks via `SetWindowsHookEx`.
- The post-recording screen offers a streamlined "Export with Auto-Zoom" flow that skips the timeline editor entirely.
- Inspector fields are read-only for now (display only, no editing).
- Timeline tracks load from the Rust backend via `get_timeline` — no longer hardcoded seed data.
