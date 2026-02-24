pub mod core;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use tauri::State;

use core::capture::create_capture;
use core::permissions::{create_permissions_manager, PermissionReport};
use core::capture::CaptureSource;
use core::project::Project;
use core::recorder::{RecordingCoordinator, RecordingStatus};
use core::render::{ExportProgress, FrameBuffer};

struct AppState {
    recorder: Mutex<RecordingCoordinator>,
    export_progress: Arc<Mutex<Option<ExportProgress>>>,
    /// Currently loaded project (set after recording or opening a project)
    current_project: Mutex<Option<LoadedProject>>,
}

/// A project loaded in the editor, with its package directory path.
#[derive(Clone)]
struct LoadedProject {
    project: Project,
    package_dir: PathBuf,
}

/// Serializable project info returned to the frontend
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct ProjectInfo {
    name: String,
    duration: f64,
    frame_rate: f64,
    width: f64,
    height: f64,
    package_path: String,
}

#[tauri::command]
fn check_permissions() -> PermissionReport {
    let manager = create_permissions_manager();
    manager.check_all()
}

#[tauri::command]
fn list_capture_sources() -> Result<Vec<CaptureSource>, String> {
    let capture = create_capture();
    capture.enumerate_sources().map_err(|e| e.to_string())
}

#[tauri::command]
fn get_recording_status(state: State<AppState>) -> RecordingStatus {
    let recorder = state.recorder.lock().unwrap();
    recorder.status()
}

#[tauri::command]
fn start_recording(state: State<AppState>) -> Result<(), String> {
    let mut recorder = state.recorder.lock().unwrap();
    recorder.start().map_err(|e| e.to_string())
}

#[tauri::command]
fn pause_recording(state: State<AppState>) -> Result<(), String> {
    let mut recorder = state.recorder.lock().unwrap();
    recorder.pause().map_err(|e| e.to_string())
}

#[tauri::command]
fn resume_recording(state: State<AppState>) -> Result<(), String> {
    let mut recorder = state.recorder.lock().unwrap();
    recorder.resume().map_err(|e| e.to_string())
}

#[tauri::command]
fn stop_recording(state: State<AppState>) -> Result<ProjectInfo, String> {
    let mut recorder = state.recorder.lock().unwrap();
    let result = recorder.stop().map_err(|e| e.to_string())?;

    // Save input data alongside video
    let mouse_path = result.save_input_data().map_err(|e| e.to_string())?;

    // Create and save project package
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let package_name = format!("Recording_{timestamp}.lazyrec");
    let package_dir = result.video_path.parent()
        .unwrap_or(std::path::Path::new("."))
        .join(&package_name);

    let mut project = result.to_project(format!("Recording {timestamp}"));
    project.save(
        &package_dir,
        Some(&result.video_path),
        Some(&mouse_path),
    ).map_err(|e| e.to_string())?;

    let info = ProjectInfo {
        name: project.name.clone(),
        duration: project.duration(),
        frame_rate: project.media.frame_rate,
        width: project.media.pixel_size.width,
        height: project.media.pixel_size.height,
        package_path: package_dir.display().to_string(),
    };

    // Store as current project
    {
        let mut current = state.current_project.lock().unwrap();
        *current = Some(LoadedProject {
            project,
            package_dir,
        });
    }

    // Reset for next recording
    recorder.reset();

    Ok(info)
}

#[tauri::command]
fn start_export(state: State<AppState>) -> Result<String, String> {
    use core::render::{ExportEngine, create_video_source_from_file};

    let current = state.current_project.lock().unwrap();
    let loaded = current.as_ref().ok_or("No project loaded. Record or open a project first.")?;
    let project = &loaded.project;

    // Output path: same directory as the package, timestamped
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let output_dir = loaded.package_dir.parent()
        .unwrap_or(std::path::Path::new("."));
    let output_path = output_dir.join(format!("export_{timestamp}.mp4"));

    // Create video source from the project's video file (falls back to stub)
    let video_path = project.video_path(&loaded.package_dir);
    let source = create_video_source_from_file(
        &video_path,
        project.media.pixel_size.width as u32,
        project.media.pixel_size.height as u32,
        project.duration(),
        project.media.frame_rate,
    );

    // Load mouse positions for the evaluator
    let mouse_positions = {
        let mouse_path = project.mouse_data_path(&loaded.package_dir);
        if mouse_path.exists() {
            let json = std::fs::read_to_string(&mouse_path).unwrap_or_default();
            core::input::InputRecording::from_json(&json)
                .map(|r| input_to_evaluator_positions(&r))
                .unwrap_or_default()
        } else {
            vec![]
        }
    };

    let mut engine = ExportEngine::from_project(
        project,
        source,
        mouse_positions,
        output_path,
    );

    // Release the lock before the long export operation
    drop(current);

    let progress_state = state.export_progress.clone();
    let result = engine.export(move |progress| {
        if let Ok(mut p) = progress_state.lock() {
            *p = Some(progress);
        }
    });

    match result {
        Ok(path) => {
            let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
            let size_mb = size as f64 / (1024.0 * 1024.0);
            Ok(format!("Export complete: {} ({:.1} MB)", path.display(), size_mb))
        }
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
fn get_export_progress(state: State<AppState>) -> Option<ExportProgress> {
    state.export_progress.lock().unwrap().clone()
}

/// Save the current project to its package directory.
#[tauri::command]
fn save_project(state: State<AppState>) -> Result<String, String> {
    let mut current = state.current_project.lock().unwrap();
    let loaded = current.as_mut().ok_or("No project loaded")?;

    loaded.project.save(&loaded.package_dir, None, None)
        .map_err(|e| e.to_string())?;

    Ok(format!("Project saved: {}", loaded.package_dir.display()))
}

/// Load a project from a `.lazyrec` package directory path.
#[tauri::command]
fn load_project(path: String, state: State<AppState>) -> Result<ProjectInfo, String> {
    let package_dir = PathBuf::from(&path);
    let project = Project::load(&package_dir).map_err(|e| e.to_string())?;

    let info = ProjectInfo {
        name: project.name.clone(),
        duration: project.duration(),
        frame_rate: project.media.frame_rate,
        width: project.media.pixel_size.width,
        height: project.media.pixel_size.height,
        package_path: package_dir.display().to_string(),
    };

    let mut current = state.current_project.lock().unwrap();
    *current = Some(LoadedProject {
        project,
        package_dir,
    });

    Ok(info)
}

/// Serializable mouse position for the frontend
#[derive(serde::Serialize, serde::Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct MousePositionData {
    time: f64,
    x: f64,
    y: f64,
}

/// Load mouse data from the current project.
/// Returns mouse positions as normalized (0-1) coordinates.
#[tauri::command]
fn load_mouse_data(state: State<AppState>) -> Result<Vec<MousePositionData>, String> {
    let current = state.current_project.lock().unwrap();
    let loaded = current.as_ref().ok_or("No project loaded")?;

    let mouse_path = loaded.project.mouse_data_path(&loaded.package_dir);
    if !mouse_path.exists() {
        return Ok(vec![]);
    }

    let json = std::fs::read_to_string(&mouse_path).map_err(|e| e.to_string())?;
    let recording = core::input::InputRecording::from_json(&json)
        .map_err(|e| e.to_string())?;

    let positions: Vec<MousePositionData> = recording.positions.iter().map(|p| {
        MousePositionData {
            time: p.time,
            x: p.position.x,
            y: p.position.y,
        }
    }).collect();

    Ok(positions)
}

/// Convert InputRecording to generator MouseData format
fn input_to_mouse_data(recording: &core::input::InputRecording, duration: f64) -> core::generators::MouseData {
    use core::generators::*;
    use core::input::MouseButton;

    let positions: Vec<(f64, core::coordinates::NormalizedPoint)> = recording
        .positions.iter().map(|p| (p.time, p.position)).collect();

    let clicks: Vec<ClickEvent> = recording.clicks.iter().map(|c| {
        let click_type = match c.button {
            MouseButton::Left => ClickType::LeftDown,
            MouseButton::Right => ClickType::RightDown,
            MouseButton::Middle => ClickType::LeftDown, // treat middle as left for generators
        };
        ClickEvent {
            time: c.time,
            position: c.position,
            click_type,
            duration: c.duration,
        }
    }).collect();

    let keyboard_events: Vec<KeyboardEvent> = recording.keyboard.iter().map(|k| {
        KeyboardEvent {
            time: k.time,
            event_type: match k.event_type {
                core::input::KeyAction::Down => KeyEventType::KeyDown,
                core::input::KeyAction::Up => KeyEventType::KeyUp,
            },
            key_code: k.key_code,
            character: k.character.clone(),
            modifiers: Modifiers {
                command: k.modifiers.command,
                shift: k.modifiers.shift,
                alt: k.modifiers.alt,
                control: k.modifiers.control,
            },
        }
    }).collect();

    let drags: Vec<DragEvent> = recording.drags.iter().map(|d| {
        DragEvent {
            start_time: d.start_time,
            end_time: d.end_time,
            start_position: d.start_position,
            end_position: d.end_position,
        }
    }).collect();

    MouseData {
        positions,
        clicks,
        keyboard_events,
        drags,
        duration,
    }
}

/// Convert InputRecording positions to evaluator MousePosition format
fn input_to_evaluator_positions(recording: &core::input::InputRecording) -> Vec<core::evaluator::MousePosition> {
    recording.positions.iter().map(|p| {
        core::evaluator::MousePosition {
            time: p.time,
            position: p.position,
        }
    }).collect()
}

/// Generated keyframes result returned to the frontend
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct GeneratedKeyframes {
    transform_count: usize,
    ripple_count: usize,
    cursor_count: usize,
    keystroke_count: usize,
    total: usize,
}

/// Run auto-generators on the current project's mouse/keyboard data.
/// Replaces the project's timeline tracks with generated keyframes.
#[tauri::command]
fn generate_keyframes(state: State<AppState>) -> Result<GeneratedKeyframes, String> {
    use core::generators::*;
    use core::track::AnyTrack;

    let mut current = state.current_project.lock().unwrap();
    let loaded = current.as_mut().ok_or("No project loaded")?;

    // Load mouse data
    let mouse_path = loaded.project.mouse_data_path(&loaded.package_dir);
    let recording = if mouse_path.exists() {
        let json = std::fs::read_to_string(&mouse_path).map_err(|e| e.to_string())?;
        core::input::InputRecording::from_json(&json).map_err(|e| e.to_string())?
    } else {
        return Err("No mouse data found in project".into());
    };

    let duration = loaded.project.duration();
    let mouse_data = input_to_mouse_data(&recording, duration);

    // Run generators
    let zoom_settings = SmartZoomSettings::default();
    let ripple_settings = RippleSettings::default();
    let keystroke_settings = KeystrokeSettings::default();

    let transform_track = generate_smart_zoom(&mouse_data, &zoom_settings);
    let ripple_track = generate_ripples(&mouse_data.clicks, &ripple_settings);
    let keystroke_track = generate_keystrokes(&mouse_data.keyboard_events, &keystroke_settings);
    let cursor_track = generate_cursor_keyframes(&mouse_data.positions, &mouse_data.clicks);

    let result = GeneratedKeyframes {
        transform_count: transform_track.keyframe_count(),
        ripple_count: ripple_track.keyframe_count(),
        cursor_count: cursor_track.style_keyframes.as_ref().map_or(0, |v| v.len()),
        keystroke_count: keystroke_track.keyframe_count(),
        total: transform_track.keyframe_count()
            + ripple_track.keyframe_count()
            + cursor_track.style_keyframes.as_ref().map_or(0, |v| v.len())
            + keystroke_track.keyframe_count(),
    };

    // Replace tracks in the project timeline
    loaded.project.timeline.tracks = vec![
        AnyTrack::Transform(transform_track),
        AnyTrack::Ripple(ripple_track),
        AnyTrack::Cursor(cursor_track),
        AnyTrack::Keystroke(keystroke_track),
    ];

    // Auto-save the updated project
    loaded.project.save(&loaded.package_dir, None, None)
        .map_err(|e| e.to_string())?;

    Ok(result)
}

/// Get the currently loaded project info.
#[tauri::command]
fn get_current_project(state: State<AppState>) -> Option<ProjectInfo> {
    let current = state.current_project.lock().unwrap();
    current.as_ref().map(|loaded| ProjectInfo {
        name: loaded.project.name.clone(),
        duration: loaded.project.duration(),
        frame_rate: loaded.project.media.frame_rate,
        width: loaded.project.media.pixel_size.width,
        height: loaded.project.media.pixel_size.height,
        package_path: loaded.package_dir.display().to_string(),
    })
}

/// Frame data returned to the frontend for preview rendering.
/// Contains base64-encoded RGBA pixel data and dimensions.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct FrameData {
    width: u32,
    height: u32,
    /// Base64-encoded RGBA pixel data (width * height * 4 bytes)
    rgba_base64: String,
}

/// Extract a video frame at the given time for preview.
/// Uses the stub video source (or FFmpeg when available).
/// Throttled by the frontend to avoid excessive calls during scrubbing.
#[tauri::command]
fn extract_preview_frame(time: f64) -> Result<FrameData, String> {
    use core::render::create_video_source;
    use base64::Engine;

    // Create a video source (stub or FFmpeg in the future).
    // In production, this would use the project's actual video file.
    let mut source = create_video_source(640, 360, 30.0, 30.0);

    let frame = source.read_frame(time).map_err(|e| e.to_string())?;

    // Convert BGRA → RGBA for HTML Canvas ImageData
    let rgba = bgra_to_rgba(&frame);

    let rgba_base64 = base64::engine::general_purpose::STANDARD.encode(&rgba);

    Ok(FrameData {
        width: frame.width,
        height: frame.height,
        rgba_base64,
    })
}

/// Convert BGRA pixel data to RGBA for use with HTML Canvas ImageData
fn bgra_to_rgba(frame: &FrameBuffer) -> Vec<u8> {
    let mut rgba = vec![0u8; frame.data.len()];
    for (src, dst) in frame.data.chunks_exact(4).zip(rgba.chunks_exact_mut(4)) {
        dst[0] = src[2]; // R
        dst[1] = src[1]; // G
        dst[2] = src[0]; // B
        dst[3] = src[3]; // A
    }
    rgba
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let output_dir = dirs::video_dir()
        .unwrap_or_else(|| dirs::home_dir().unwrap_or_default().join("Videos"))
        .join("LazyRec");

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState {
            recorder: Mutex::new(RecordingCoordinator::new(output_dir)),
            export_progress: Arc::new(Mutex::new(None)),
            current_project: Mutex::new(None),
        })
        .invoke_handler(tauri::generate_handler![
            check_permissions,
            list_capture_sources,
            get_recording_status,
            start_recording,
            pause_recording,
            resume_recording,
            stop_recording,
            start_export,
            get_export_progress,
            extract_preview_frame,
            save_project,
            load_project,
            get_current_project,
            load_mouse_data,
            generate_keyframes,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
