//! Screen capture abstraction trait.
//! Platform-specific implementations go in platform modules.

use serde::{Deserialize, Serialize};

/// Capture target selection
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum CaptureTarget {
    /// Capture entire display
    #[serde(rename = "display")]
    Display { display_id: u32 },
    /// Capture a specific window
    #[serde(rename = "window")]
    Window { window_id: u64, title: String },
    /// Capture a rectangular region of a display
    #[serde(rename = "region")]
    Region {
        display_id: u32,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
    },
}

/// Enumerated capture source
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureSource {
    pub id: String,
    pub name: String,
    pub source_type: CaptureSourceType,
    /// Width in pixels
    pub width: u32,
    /// Height in pixels
    pub height: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CaptureSourceType {
    Display,
    Window,
}

/// Capture configuration
#[derive(Debug, Clone)]
pub struct CaptureConfig {
    pub target_fps: u32,
    pub pixel_format: PixelFormat,
    /// Exclude the app's own windows from capture
    pub exclude_self: bool,
}

impl Default for CaptureConfig {
    fn default() -> Self {
        Self {
            target_fps: 60,
            pixel_format: PixelFormat::Bgra8,
            exclude_self: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelFormat {
    Bgra8,
    Rgba8,
    Nv12,
}

/// A single captured video frame
pub struct CapturedFrame {
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub stride: u32,
    pub pixel_format: PixelFormat,
    /// Frame presentation timestamp in seconds (relative to capture start)
    pub timestamp: f64,
}

/// Capture error types
#[derive(Debug, thiserror::Error)]
pub enum CaptureError {
    #[error("Already capturing")]
    AlreadyCapturing,
    #[error("Not currently capturing")]
    NotCapturing,
    #[error("Permission denied for screen capture")]
    PermissionDenied,
    #[error("Capture target not found")]
    TargetNotFound,
    #[error("Configuration failed: {0}")]
    ConfigurationFailed(String),
    #[error("Platform error: {0}")]
    Platform(String),
}

/// Screen capture abstraction trait.
/// Each platform provides its own implementation.
pub trait ScreenCapture: Send {
    /// Enumerate available capture sources (displays and windows)
    fn enumerate_sources(&self) -> Result<Vec<CaptureSource>, CaptureError>;

    /// Start capturing frames from the given target.
    /// Frames will be delivered via the callback.
    fn start_capture(
        &mut self,
        target: CaptureTarget,
        config: CaptureConfig,
        on_frame: Box<dyn FnMut(CapturedFrame) + Send>,
    ) -> Result<(), CaptureError>;

    /// Stop the current capture session.
    fn stop_capture(&mut self) -> Result<(), CaptureError>;

    /// Check if currently capturing
    fn is_capturing(&self) -> bool;
}

// Stub implementation for development/testing
#[cfg(not(any(target_os = "windows", target_os = "linux")))]
pub mod stub {
    use super::*;

    pub struct StubCapture {
        capturing: bool,
    }

    impl StubCapture {
        pub fn new() -> Self {
            Self { capturing: false }
        }
    }

    impl ScreenCapture for StubCapture {
        fn enumerate_sources(&self) -> Result<Vec<CaptureSource>, CaptureError> {
            Ok(vec![CaptureSource {
                id: "stub-display-0".into(),
                name: "Primary Display (Stub)".into(),
                source_type: CaptureSourceType::Display,
                width: 1920,
                height: 1080,
            }])
        }

        fn start_capture(
            &mut self,
            _target: CaptureTarget,
            _config: CaptureConfig,
            _on_frame: Box<dyn FnMut(CapturedFrame) + Send>,
        ) -> Result<(), CaptureError> {
            if self.capturing {
                return Err(CaptureError::AlreadyCapturing);
            }
            self.capturing = true;
            Ok(())
        }

        fn stop_capture(&mut self) -> Result<(), CaptureError> {
            if !self.capturing {
                return Err(CaptureError::NotCapturing);
            }
            self.capturing = false;
            Ok(())
        }

        fn is_capturing(&self) -> bool {
            self.capturing
        }
    }
}

/// Platform-specific capture implementations
#[cfg(target_os = "windows")]
pub mod windows {
    use super::*;

    pub struct WindowsCapture {
        capturing: bool,
    }

    impl WindowsCapture {
        pub fn new() -> Self {
            Self { capturing: false }
        }
    }

    impl ScreenCapture for WindowsCapture {
        fn enumerate_sources(&self) -> Result<Vec<CaptureSource>, CaptureError> {
            // TODO: Use DXGI to enumerate outputs and EnumWindows for windows
            Ok(vec![CaptureSource {
                id: "display-0".into(),
                name: "Primary Display".into(),
                source_type: CaptureSourceType::Display,
                width: 1920,
                height: 1080,
            }])
        }

        fn start_capture(
            &mut self,
            _target: CaptureTarget,
            _config: CaptureConfig,
            _on_frame: Box<dyn FnMut(CapturedFrame) + Send>,
        ) -> Result<(), CaptureError> {
            if self.capturing {
                return Err(CaptureError::AlreadyCapturing);
            }
            // TODO: Initialize DXGI Desktop Duplication or Windows.Graphics.Capture
            self.capturing = true;
            Ok(())
        }

        fn stop_capture(&mut self) -> Result<(), CaptureError> {
            if !self.capturing {
                return Err(CaptureError::NotCapturing);
            }
            // TODO: Release DXGI resources
            self.capturing = false;
            Ok(())
        }

        fn is_capturing(&self) -> bool {
            self.capturing
        }
    }
}

#[cfg(target_os = "linux")]
pub mod linux {
    use super::*;

    pub struct LinuxCapture {
        capturing: bool,
    }

    impl LinuxCapture {
        pub fn new() -> Self {
            Self { capturing: false }
        }
    }

    impl ScreenCapture for LinuxCapture {
        fn enumerate_sources(&self) -> Result<Vec<CaptureSource>, CaptureError> {
            // TODO: Use PipeWire or X11 to enumerate displays/windows
            Ok(vec![CaptureSource {
                id: "display-0".into(),
                name: "Primary Display".into(),
                source_type: CaptureSourceType::Display,
                width: 1920,
                height: 1080,
            }])
        }

        fn start_capture(
            &mut self,
            _target: CaptureTarget,
            _config: CaptureConfig,
            _on_frame: Box<dyn FnMut(CapturedFrame) + Send>,
        ) -> Result<(), CaptureError> {
            if self.capturing {
                return Err(CaptureError::AlreadyCapturing);
            }
            // TODO: Initialize PipeWire stream or X11 SHM capture
            self.capturing = true;
            Ok(())
        }

        fn stop_capture(&mut self) -> Result<(), CaptureError> {
            if !self.capturing {
                return Err(CaptureError::NotCapturing);
            }
            self.capturing = false;
            Ok(())
        }

        fn is_capturing(&self) -> bool {
            self.capturing
        }
    }
}

/// Create the platform-appropriate capture backend
pub fn create_capture() -> Box<dyn ScreenCapture> {
    #[cfg(target_os = "windows")]
    { Box::new(windows::WindowsCapture::new()) }

    #[cfg(target_os = "linux")]
    { Box::new(linux::LinuxCapture::new()) }

    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    { Box::new(stub::StubCapture::new()) }
}
