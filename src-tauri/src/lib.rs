pub mod core;

use std::sync::Mutex;

use tauri::State;

use core::capture::create_capture;
use core::permissions::{create_permissions_manager, PermissionReport};
use core::capture::CaptureSource;
use core::recorder::{RecordingCoordinator, RecordingStatus};

struct AppState {
    recorder: Mutex<RecordingCoordinator>,
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let output_dir = dirs::video_dir()
        .unwrap_or_else(|| dirs::home_dir().unwrap_or_default().join("Videos"))
        .join("LazyRec");

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState {
            recorder: Mutex::new(RecordingCoordinator::new(output_dir)),
        })
        .invoke_handler(tauri::generate_handler![
            check_permissions,
            list_capture_sources,
            get_recording_status,
            start_recording,
            pause_recording,
            resume_recording,
            stop_recording,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
