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
    use core::coordinates::NormalizedPoint;
    use core::evaluator::MousePosition;
    use core::project::{CaptureMeta, MediaAsset, Project, Rect, Size};
    use core::render::{ExportEngine, create_video_source};

    let output_dir = dirs::video_dir()
        .unwrap_or_else(|| dirs::home_dir().unwrap_or_default().join("Videos"))
        .join("LazyRec");

    std::fs::create_dir_all(&output_dir).map_err(|e| e.to_string())?;

    let output_path = output_dir.join("export.mp4");

    // Create a demo project for export (in production, this comes from the editor state)
    let media = MediaAsset {
        video_relative_path: "recording.mp4".into(),
        mouse_data_relative_path: "recording_mouse.json".into(),
        pixel_size: Size::new(1920.0, 1080.0),
        frame_rate: 30.0,
        duration: 3.0,
    };

    let capture_meta = CaptureMeta::new(
        Rect::new(0.0, 0.0, 1920.0, 1080.0),
        1.0,
    );

    let project = Project::new("Export".into(), media, capture_meta);

    let source = create_video_source(1920, 1080, 3.0, 30.0);
    let mouse_positions = vec![
        MousePosition { time: 0.0, position: NormalizedPoint::new(0.3, 0.4) },
        MousePosition { time: 1.5, position: NormalizedPoint::new(0.6, 0.5) },
        MousePosition { time: 3.0, position: NormalizedPoint::new(0.7, 0.6) },
    ];

    let mut engine = ExportEngine::from_project(
        &project,
        source,
        mouse_positions,
        output_path,
    );

    let progress_state = state.export_progress.clone();
    let result = engine.export(move |progress| {
        if let Ok(mut p) = progress_state.lock() {
            *p = Some(progress);
        }
    });

    match result {
        Ok(path) => Ok(format!("Export complete: {}", path.display())),
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
