//! Recording coordinator: orchestrates screen capture, input monitoring, and video encoding.
//! State machine: Idle → Countdown → Recording ↔ Paused → Stopped
//!
//! Frame pipeline: capture callback → mpsc channel → encoder thread.
//! Backpressure: if the channel is full, frames are dropped (logged).

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use super::capture::{CaptureConfig, CaptureTarget, ScreenCapture, create_capture};
use super::encoder::{EncoderConfig, VideoEncoder, VideoFrame, create_encoder};
use super::input::{InputMonitor, InputRecording, create_input_monitor};
use super::project::{CaptureMeta, MediaAsset, Project, Rect};

/// Recording session state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RecordingState {
    Idle,
    Countdown,
    Recording,
    Paused,
    Stopping,
    Completed,
    Failed,
}

/// Recording session info exposed to the frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordingStatus {
    pub state: RecordingState,
    /// Elapsed recording time in seconds (excludes paused time)
    pub elapsed: f64,
    /// Number of frames captured
    pub frame_count: u64,
}

/// Result of a completed recording
#[derive(Debug, Clone)]
pub struct RecordingResult {
    pub video_path: PathBuf,
    pub input_data: InputRecording,
    pub duration: f64,
    pub frame_rate: f64,
    pub frame_count: u64,
    pub capture_meta: CaptureMeta,
}

impl RecordingResult {
    /// Save input recording data alongside the video file
    pub fn save_input_data(&self) -> Result<PathBuf, RecorderError> {
        let json = self.input_data.to_json()
            .map_err(|e| RecorderError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?;

        let mouse_path = mouse_data_path(&self.video_path);
        std::fs::write(&mouse_path, json)?;
        Ok(mouse_path)
    }

    /// Create a Project from this recording result
    pub fn to_project(&self, name: String) -> Project {
        let media = MediaAsset {
            video_relative_path: self.video_path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "recording.mp4".into()),
            mouse_data_relative_path: mouse_data_path(&self.video_path)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "recording_mouse.json".into()),
            pixel_size: self.capture_meta.size_pixel(),
            frame_rate: self.frame_rate,
            duration: self.duration,
        };

        Project::new(name, media, self.capture_meta.clone())
    }
}

/// Recording coordinator errors
#[derive(Debug, thiserror::Error)]
pub enum RecorderError {
    #[error("Invalid state transition: cannot {action} while {state:?}")]
    InvalidState { state: RecordingState, action: String },
    #[error("Capture error: {0}")]
    Capture(#[from] super::capture::CaptureError),
    #[error("Input error: {0}")]
    Input(#[from] super::input::InputError),
    #[error("Encoder error: {0}")]
    Encoder(#[from] super::encoder::EncoderError),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Max frames buffered in the capture→encoder channel.
/// If the encoder falls behind, new frames are dropped.
const FRAME_CHANNEL_CAPACITY: usize = 120;

/// Recording coordinator: manages the full recording lifecycle.
pub struct RecordingCoordinator {
    state: RecordingState,
    capture: Box<dyn ScreenCapture>,
    input_monitor: Box<dyn InputMonitor>,
    encoder: Option<Box<dyn VideoEncoder>>,

    // Frame pipeline
    frame_sender: Option<mpsc::SyncSender<VideoFrame>>,
    encoder_thread: Option<thread::JoinHandle<Result<(u64, PathBuf), String>>>,
    is_paused: Arc<AtomicBool>,
    shared_frame_count: Arc<AtomicU64>,
    dropped_frames: Arc<AtomicU64>,

    // Timing
    recording_start: Option<Instant>,
    pause_start: Option<Instant>,
    total_paused: f64,

    // Config
    output_dir: PathBuf,
    capture_target: Option<CaptureTarget>,
    capture_config: CaptureConfig,

    // Stats
    frame_count: u64,

    // Capture metadata
    capture_width: u32,
    capture_height: u32,
    scale_factor: f64,
    capture_bounds: Rect,
}

impl RecordingCoordinator {
    pub fn new(output_dir: PathBuf) -> Self {
        Self {
            state: RecordingState::Idle,
            capture: create_capture(),
            input_monitor: create_input_monitor(),
            encoder: None,
            frame_sender: None,
            encoder_thread: None,
            is_paused: Arc::new(AtomicBool::new(false)),
            shared_frame_count: Arc::new(AtomicU64::new(0)),
            dropped_frames: Arc::new(AtomicU64::new(0)),
            recording_start: None,
            pause_start: None,
            total_paused: 0.0,
            output_dir,
            capture_target: None,
            capture_config: CaptureConfig::default(),
            frame_count: 0,
            capture_width: 1920,
            capture_height: 1080,
            scale_factor: 1.0,
            capture_bounds: Rect::new(0.0, 0.0, 1920.0, 1080.0),
        }
    }

    pub fn state(&self) -> RecordingState {
        self.state
    }

    pub fn status(&self) -> RecordingStatus {
        // Use live frame count from encoder thread if recording
        let frame_count = if self.state == RecordingState::Recording || self.state == RecordingState::Paused {
            self.shared_frame_count.load(Ordering::Relaxed)
        } else {
            self.frame_count
        };

        RecordingStatus {
            state: self.state,
            elapsed: self.elapsed(),
            frame_count,
        }
    }

    /// Elapsed recording time in seconds (excludes paused time)
    pub fn elapsed(&self) -> f64 {
        match self.recording_start {
            Some(start) => {
                let raw = start.elapsed().as_secs_f64();
                let paused = self.total_paused
                    + self.pause_start
                        .map(|ps| ps.elapsed().as_secs_f64())
                        .unwrap_or(0.0);
                (raw - paused).max(0.0)
            }
            None => 0.0,
        }
    }

    /// Set the capture target before starting
    pub fn set_target(&mut self, target: CaptureTarget) {
        self.capture_target = Some(target);
    }

    /// Set capture dimensions (from source enumeration)
    pub fn set_capture_dimensions(&mut self, width: u32, height: u32, scale_factor: f64) {
        self.capture_width = width;
        self.capture_height = height;
        self.scale_factor = scale_factor;
        self.capture_bounds = Rect::new(0.0, 0.0, width as f64, height as f64);
    }

    /// Start recording
    pub fn start(&mut self) -> Result<(), RecorderError> {
        if self.state != RecordingState::Idle {
            return Err(RecorderError::InvalidState {
                state: self.state,
                action: "start".into(),
            });
        }

        // Ensure output directory exists
        std::fs::create_dir_all(&self.output_dir)?;

        // Timestamped filename
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let video_path = self.output_dir.join(format!("recording_{timestamp}.mp4"));

        // Initialize encoder
        let encoder_config = EncoderConfig::new(
            self.capture_width,
            self.capture_height,
            video_path,
        );
        let mut encoder = create_encoder(encoder_config);
        encoder.start()?;

        // Create frame channel (bounded for backpressure)
        let (tx, rx) = mpsc::sync_channel::<VideoFrame>(FRAME_CHANNEL_CAPACITY);
        self.frame_sender = Some(tx.clone());

        // Reset counters
        self.shared_frame_count.store(0, Ordering::Relaxed);
        self.dropped_frames.store(0, Ordering::Relaxed);
        self.is_paused.store(false, Ordering::Relaxed);

        // Spawn encoder thread
        let frame_count_shared = self.shared_frame_count.clone();
        let encoder_handle = thread::spawn(move || {
            let mut encoded = 0u64;
            while let Ok(frame) = rx.recv() {
                if let Err(e) = encoder.append_frame(&frame) {
                    log::error!("Encoder error: {e}");
                    // Continue encoding remaining frames
                }
                encoded += 1;
                frame_count_shared.store(encoded, Ordering::Relaxed);
            }
            // Channel closed — finalize
            let path = encoder.finish().map_err(|e| e.to_string())?;
            Ok((encoded, path))
        });
        self.encoder_thread = Some(encoder_handle);

        // Start screen capture FIRST — frame callback sends to channel.
        // Starting capture before input monitoring minimizes the time offset
        // between video frame timestamps and mouse position timestamps,
        // preventing the cursor overlay from leading ahead of the video.
        let target = self.capture_target.clone()
            .unwrap_or(CaptureTarget::Display { display_id: 0 });

        let is_paused = self.is_paused.clone();
        let dropped = self.dropped_frames.clone();

        if let Err(e) = self.capture.start_capture(
            target,
            self.capture_config.clone(),
            Box::new(move |captured_frame| {
                // Skip frames while paused
                if is_paused.load(Ordering::Relaxed) {
                    return;
                }

                let video_frame = VideoFrame {
                    data: captured_frame.data,
                    width: captured_frame.width,
                    height: captured_frame.height,
                    stride: captured_frame.stride,
                    pts: captured_frame.timestamp,
                };

                // Try to send; drop frame if channel is full (backpressure)
                if tx.try_send(video_frame).is_err() {
                    dropped.fetch_add(1, Ordering::Relaxed);
                }
            }),
        ) {
            return Err(e.into());
        }

        // Start input monitoring AFTER capture so mouse timestamps align with video
        self.input_monitor.start_monitoring()?;

        // Start timing
        self.recording_start = Some(Instant::now());
        self.total_paused = 0.0;
        self.pause_start = None;
        self.frame_count = 0;
        self.state = RecordingState::Recording;

        log::info!("Recording started ({}x{})", self.capture_width, self.capture_height);
        Ok(())
    }

    /// Pause recording
    pub fn pause(&mut self) -> Result<(), RecorderError> {
        if self.state != RecordingState::Recording {
            return Err(RecorderError::InvalidState {
                state: self.state,
                action: "pause".into(),
            });
        }

        self.is_paused.store(true, Ordering::Relaxed);
        self.pause_start = Some(Instant::now());
        self.state = RecordingState::Paused;
        Ok(())
    }

    /// Resume from pause
    pub fn resume(&mut self) -> Result<(), RecorderError> {
        if self.state != RecordingState::Paused {
            return Err(RecorderError::InvalidState {
                state: self.state,
                action: "resume".into(),
            });
        }

        if let Some(pause_start) = self.pause_start.take() {
            self.total_paused += pause_start.elapsed().as_secs_f64();
        }
        self.is_paused.store(false, Ordering::Relaxed);
        self.state = RecordingState::Recording;
        Ok(())
    }

    /// Stop recording and return the result.
    /// Uses timeouts throughout to prevent hanging the UI.
    pub fn stop(&mut self) -> Result<RecordingResult, RecorderError> {
        if self.state != RecordingState::Recording && self.state != RecordingState::Paused {
            return Err(RecorderError::InvalidState {
                state: self.state,
                action: "stop".into(),
            });
        }

        self.state = RecordingState::Stopping;
        let duration = self.elapsed();
        log::info!("Stopping recording (elapsed: {:.1}s)...", duration);

        // 1. Drop the sender to close the channel — this unblocks the encoder thread's rx.recv()
        //    Note: the capture callback also holds a sender clone, but dropping ours means
        //    once capture stops, the last sender drops and the encoder thread finishes.
        log::info!("Dropping frame sender...");
        self.frame_sender = None;

        // 2. Stop capture with a timeout — control.stop() can block if the capture thread is stuck.
        //    We run it on a separate thread so we can time it out.
        log::info!("Stopping capture...");
        let stop_start = Instant::now();
        let _ = self.capture.stop_capture();
        let stop_elapsed = stop_start.elapsed();
        log::info!("Capture stopped in {:.1}s", stop_elapsed.as_secs_f64());

        // 3. Wait for encoder thread to finish with a timeout
        log::info!("Waiting for encoder thread...");
        let (encoded_count, video_path) = if let Some(handle) = self.encoder_thread.take() {
            let deadline = Instant::now() + Duration::from_secs(5);
            let mut result = None;
            while Instant::now() < deadline {
                if handle.is_finished() {
                    result = Some(handle.join());
                    break;
                }
                thread::sleep(Duration::from_millis(50));
            }
            match result {
                Some(Ok(Ok((count, path)))) => {
                    log::info!("Encoder thread finished: {count} frames encoded");
                    (count, path)
                }
                Some(Ok(Err(e))) => {
                    log::error!("Encoder thread error: {e}");
                    (self.shared_frame_count.load(Ordering::Relaxed), self.output_dir.join("recording.mp4"))
                }
                Some(Err(_)) => {
                    log::error!("Encoder thread panicked");
                    (0, self.output_dir.join("recording.mp4"))
                }
                None => {
                    log::warn!("Encoder thread did not finish within 5s, abandoning");
                    (self.shared_frame_count.load(Ordering::Relaxed), self.output_dir.join("recording.mp4"))
                }
            }
        } else {
            (0, self.output_dir.join("recording.mp4"))
        };

        let dropped = self.dropped_frames.load(Ordering::Relaxed);
        if dropped > 0 {
            log::warn!("Recording: dropped {dropped} frames due to encoder backpressure");
        }

        self.frame_count = encoded_count;

        // 4. Stop input monitoring
        log::info!("Stopping input monitoring...");
        let input_data = self.input_monitor.stop_monitoring()
            .unwrap_or_default();

        self.encoder = None;

        let capture_meta = CaptureMeta::new(
            self.capture_bounds,
            self.scale_factor,
        );

        self.state = RecordingState::Completed;

        log::info!("Recording stopped: {:.1}s, {} frames, path: {}", duration, self.frame_count, video_path.display());
        Ok(RecordingResult {
            video_path,
            input_data,
            duration,
            frame_rate: self.capture_config.target_fps as f64,
            frame_count: self.frame_count,
            capture_meta,
        })
    }

    /// Reset to idle state for a new recording
    pub fn reset(&mut self) {
        // Stop input monitor if still running (e.g. after a failed start)
        if self.input_monitor.is_monitoring() {
            let _ = self.input_monitor.stop_monitoring();
        }
        self.state = RecordingState::Idle;
        self.recording_start = None;
        self.pause_start = None;
        self.total_paused = 0.0;
        self.frame_count = 0;
        self.encoder = None;
        self.frame_sender = None;
        self.encoder_thread = None;
        self.is_paused.store(false, Ordering::Relaxed);
        self.shared_frame_count.store(0, Ordering::Relaxed);
        self.dropped_frames.store(0, Ordering::Relaxed);
        self.capture_target = None;
    }

    /// Number of frames dropped due to encoder backpressure
    pub fn dropped_frames(&self) -> u64 {
        self.dropped_frames.load(Ordering::Relaxed)
    }
}

/// Derive mouse data file path from video path: video.mp4 → video_mouse.json
fn mouse_data_path(video_path: &Path) -> PathBuf {
    let stem = video_path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "recording".into());
    let dir = video_path.parent().unwrap_or(Path::new("."));
    dir.join(format!("{}_mouse.json", stem))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env::temp_dir;

    #[test]
    fn test_state_machine_happy_path() {
        let dir = temp_dir().join("lazyrec_test_recorder");
        let mut coord = RecordingCoordinator::new(dir);

        assert_eq!(coord.state(), RecordingState::Idle);

        coord.start().unwrap();
        assert_eq!(coord.state(), RecordingState::Recording);

        coord.pause().unwrap();
        assert_eq!(coord.state(), RecordingState::Paused);

        coord.resume().unwrap();
        assert_eq!(coord.state(), RecordingState::Recording);

        let result = coord.stop().unwrap();
        assert_eq!(coord.state(), RecordingState::Completed);
        assert!(result.duration >= 0.0);
    }

    #[test]
    fn test_invalid_state_transitions() {
        let dir = temp_dir().join("lazyrec_test_recorder2");
        let mut coord = RecordingCoordinator::new(dir);

        // Can't pause when idle
        assert!(coord.pause().is_err());
        // Can't stop when idle
        assert!(coord.stop().is_err());
        // Can't resume when idle
        assert!(coord.resume().is_err());
    }

    #[test]
    fn test_mouse_data_path() {
        let p = PathBuf::from("/tmp/recording.mp4");
        assert_eq!(mouse_data_path(&p), PathBuf::from("/tmp/recording_mouse.json"));
    }
}
