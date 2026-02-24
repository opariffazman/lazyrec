pub mod core;

use std::sync::{Arc, Mutex};

use tauri::State;

use core::capture::create_capture;
use core::permissions::{create_permissions_manager, PermissionReport};
use core::capture::CaptureSource;
use core::recorder::{RecordingCoordinator, RecordingStatus};
use core::render::ExportProgress;

struct AppState {
    recorder: Mutex<RecordingCoordinator>,
    export_progress: Arc<Mutex<Option<ExportProgress>>>,
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
fn stop_recording(state: State<AppState>) -> Result<String, String> {
    let mut recorder = state.recorder.lock().unwrap();
    let result = recorder.stop().map_err(|e| e.to_string())?;

    // Save input data alongside video
    let mouse_path = result.save_input_data().map_err(|e| e.to_string())?;

    // Reset for next recording
    recorder.reset();

    Ok(format!(
        "Recording saved: {} ({:.1}s, {} frames). Mouse data: {}",
        result.video_path.display(),
        result.duration,
        result.frame_count,
        mouse_path.display(),
    ))
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
