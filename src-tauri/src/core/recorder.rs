//! Recording coordinator: orchestrates screen capture, input monitoring, and video encoding.
//! State machine: Idle → Countdown → Recording ↔ Paused → Stopped

use std::path::{Path, PathBuf};
use std::time::Instant;

use serde::{Deserialize, Serialize};

use super::capture::{CaptureConfig, CaptureTarget, ScreenCapture, create_capture};
use super::encoder::{EncoderConfig, VideoEncoder, create_encoder};
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
            frame_rate: 60.0,
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

/// Recording coordinator: manages the full recording lifecycle.
pub struct RecordingCoordinator {
    state: RecordingState,
    capture: Box<dyn ScreenCapture>,
    input_monitor: Box<dyn InputMonitor>,
    encoder: Option<Box<dyn VideoEncoder>>,

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
        RecordingStatus {
            state: self.state,
            elapsed: self.elapsed(),
            frame_count: self.frame_count,
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

        let video_path = self.output_dir.join("recording.mp4");

        // Initialize encoder
        let encoder_config = EncoderConfig::new(
            self.capture_width,
            self.capture_height,
            video_path,
        );
        let mut encoder = create_encoder(encoder_config);
        encoder.start()?;
        self.encoder = Some(encoder);

        // Start input monitoring
        self.input_monitor.start_monitoring()?;

        // Start screen capture
        let target = self.capture_target.clone()
            .unwrap_or(CaptureTarget::Display { display_id: 0 });

        self.capture.start_capture(
            target,
            self.capture_config.clone(),
            Box::new(|_frame| {
                // Frame callback: in a real implementation, this would
                // feed frames to the encoder on a dedicated thread.
                // For now, frame counting happens through the encoder.
            }),
        )?;

        // Start timing
        self.recording_start = Some(Instant::now());
        self.total_paused = 0.0;
        self.pause_start = None;
        self.frame_count = 0;
        self.state = RecordingState::Recording;

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
        self.state = RecordingState::Recording;
        Ok(())
    }

    /// Stop recording and return the result
    pub fn stop(&mut self) -> Result<RecordingResult, RecorderError> {
        if self.state != RecordingState::Recording && self.state != RecordingState::Paused {
            return Err(RecorderError::InvalidState {
                state: self.state,
                action: "stop".into(),
            });
        }

        self.state = RecordingState::Stopping;
        let duration = self.elapsed();

        // Stop capture
        let _ = self.capture.stop_capture();

        // Stop input monitoring
        let input_data = self.input_monitor.stop_monitoring()
            .unwrap_or_default();

        // Finalize encoder
        let video_path = match self.encoder.as_mut() {
            Some(enc) => {
                self.frame_count = enc.frames_encoded();
                enc.finish()?
            }
            None => self.output_dir.join("recording.mp4"),
        };
        self.encoder = None;

        let capture_meta = CaptureMeta::new(
            self.capture_bounds,
            self.scale_factor,
        );

        self.state = RecordingState::Completed;

        Ok(RecordingResult {
            video_path,
            input_data,
            duration,
            frame_count: self.frame_count,
            capture_meta,
        })
    }

    /// Reset to idle state for a new recording
    pub fn reset(&mut self) {
        self.state = RecordingState::Idle;
        self.recording_start = None;
        self.pause_start = None;
        self.total_paused = 0.0;
        self.frame_count = 0;
        self.encoder = None;
        self.capture_target = None;
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
