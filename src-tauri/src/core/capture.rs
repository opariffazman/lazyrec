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

/// Windows capture via WinRT Graphics Capture API (windows-capture crate).
/// Runs capture in a background thread with CaptureControl for non-blocking operation.
#[cfg(target_os = "windows")]
pub mod windows {
    use super::*;
    use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
    use std::sync::{Arc, Mutex};
    use std::time::Instant;

    use windows_capture::capture::{CaptureControl, Context, GraphicsCaptureApiHandler};
    use windows_capture::frame::Frame;
    use windows_capture::graphics_capture_api::InternalCaptureControl;
    use windows_capture::monitor::Monitor;
    use windows_capture::settings::{
        ColorFormat, CursorCaptureSettings, DirtyRegionSettings, DrawBorderSettings,
        MinimumUpdateIntervalSettings, SecondaryWindowSettings, Settings,
    };
    use windows_capture::window::Window;

    /// Shared state passed into the capture handler via Flags
    struct CaptureFlags {
        on_frame: Mutex<Box<dyn FnMut(CapturedFrame) + Send>>,
        start_time: Instant,
        should_stop: AtomicBool,
        frame_count: AtomicU64,
    }

    struct CaptureHandler {
        flags: Arc<CaptureFlags>,
    }

    impl GraphicsCaptureApiHandler for CaptureHandler {
        type Flags = Arc<CaptureFlags>;
        type Error = Box<dyn std::error::Error + Send + Sync>;

        fn new(ctx: Context<Self::Flags>) -> Result<Self, Self::Error> {
            Ok(Self { flags: ctx.flags })
        }

        fn on_frame_arrived(
            &mut self,
            frame: &mut Frame,
            capture_control: InternalCaptureControl,
        ) -> Result<(), Self::Error> {
            if self.flags.should_stop.load(Ordering::Relaxed) {
                capture_control.stop();
                return Ok(());
            }

            let width = frame.width();
            let height = frame.height();
            let timestamp = self.flags.start_time.elapsed().as_secs_f64();

            // Get raw pixel buffer (BGRA)
            let mut buffer = frame.buffer()?;
            let raw = buffer.as_nopadding_buffer()
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

            let captured = CapturedFrame {
                data: raw.to_vec(),
                width,
                height,
                stride: width * 4,
                pixel_format: PixelFormat::Bgra8,
                timestamp,
            };

            if let Ok(mut callback) = self.flags.on_frame.lock() {
                callback(captured);
            }

            self.flags.frame_count.fetch_add(1, Ordering::Relaxed);
            Ok(())
        }
    }

    pub struct WindowsCapture {
        capturing: bool,
        control: Option<CaptureControl<CaptureHandler, Box<dyn std::error::Error + Send + Sync>>>,
        flags: Option<Arc<CaptureFlags>>,
    }

    impl WindowsCapture {
        pub fn new() -> Self {
            Self {
                capturing: false,
                control: None,
                flags: None,
            }
        }
    }

    impl ScreenCapture for WindowsCapture {
        fn enumerate_sources(&self) -> Result<Vec<CaptureSource>, CaptureError> {
            let mut sources = Vec::new();

            // Enumerate monitors
            if let Ok(monitors) = Monitor::enumerate() {
                for (i, monitor) in monitors.iter().enumerate() {
                    let name = monitor.name().unwrap_or_else(|_| format!("Display {}", i + 1));
                    let w = monitor.width().unwrap_or(1920);
                    let h = monitor.height().unwrap_or(1080);
                    sources.push(CaptureSource {
                        id: format!("display-{}", i),
                        name,
                        source_type: CaptureSourceType::Display,
                        width: w,
                        height: h,
                    });
                }
            }

            // Enumerate windows
            // Note: Window dimensions are determined at capture time by the frame callback.
            // We use nominal defaults here since window.width()/height() are not available
            // in windows-capture 1.x.
            if let Ok(windows) = Window::enumerate() {
                for window in windows {
                    if !window.is_valid() {
                        continue;
                    }
                    let title = match window.title() {
                        Ok(t) if !t.is_empty() => t,
                        _ => continue,
                    };
                    sources.push(CaptureSource {
                        id: format!("window-{}", title.replace(' ', "_")),
                        name: title,
                        source_type: CaptureSourceType::Window,
                        width: 0,
                        height: 0,
                    });
                }
            }

            if sources.is_empty() {
                // Fallback
                sources.push(CaptureSource {
                    id: "display-0".into(),
                    name: "Primary Display".into(),
                    source_type: CaptureSourceType::Display,
                    width: 1920,
                    height: 1080,
                });
            }

            Ok(sources)
        }

        fn start_capture(
            &mut self,
            target: CaptureTarget,
            _config: CaptureConfig,
            on_frame: Box<dyn FnMut(CapturedFrame) + Send>,
        ) -> Result<(), CaptureError> {
            if self.capturing {
                return Err(CaptureError::AlreadyCapturing);
            }

            let flags = Arc::new(CaptureFlags {
                on_frame: Mutex::new(on_frame),
                start_time: Instant::now(),
                should_stop: AtomicBool::new(false),
                frame_count: AtomicU64::new(0),
            });

            // Build settings based on target
            let control = match target {
                CaptureTarget::Display { display_id } => {
                    let monitor = if display_id == 0 {
                        Monitor::primary()
                    } else {
                        Monitor::from_index(display_id as usize)
                    }
                    .map_err(|e| CaptureError::Platform(format!("Monitor not found: {e}")))?;

                    let settings = Settings::new(
                        monitor,
                        CursorCaptureSettings::WithCursor,
                        DrawBorderSettings::WithoutBorder,
                        SecondaryWindowSettings::Default,
                        MinimumUpdateIntervalSettings::Default,
                        DirtyRegionSettings::Default,
                        ColorFormat::Bgra8,
                        flags.clone(),
                    );

                    CaptureHandler::start_free_threaded(settings)
                        .map_err(|e| CaptureError::Platform(e.to_string()))?
                }
                CaptureTarget::Window { title, .. } => {
                    let window = Window::from_contains_name(&title)
                        .map_err(|e| CaptureError::Platform(format!("Window not found: {e}")))?;

                    let settings = Settings::new(
                        window,
                        CursorCaptureSettings::WithCursor,
                        DrawBorderSettings::WithoutBorder,
                        SecondaryWindowSettings::Default,
                        MinimumUpdateIntervalSettings::Default,
                        DirtyRegionSettings::Default,
                        ColorFormat::Bgra8,
                        flags.clone(),
                    );

                    CaptureHandler::start_free_threaded(settings)
                        .map_err(|e| CaptureError::Platform(e.to_string()))?
                }
                CaptureTarget::Region { display_id, .. } => {
                    // Fall back to full display capture for regions
                    // (cropping happens in the render pipeline)
                    let monitor = if display_id == 0 {
                        Monitor::primary()
                    } else {
                        Monitor::from_index(display_id as usize)
                    }
                    .map_err(|e| CaptureError::Platform(format!("Monitor not found: {e}")))?;

                    let settings = Settings::new(
                        monitor,
                        CursorCaptureSettings::WithCursor,
                        DrawBorderSettings::WithoutBorder,
                        SecondaryWindowSettings::Default,
                        MinimumUpdateIntervalSettings::Default,
                        DirtyRegionSettings::Default,
                        ColorFormat::Bgra8,
                        flags.clone(),
                    );

                    CaptureHandler::start_free_threaded(settings)
                        .map_err(|e| CaptureError::Platform(e.to_string()))?
                }
            };

            self.flags = Some(flags);
            self.control = Some(control);
            self.capturing = true;
            Ok(())
        }

        fn stop_capture(&mut self) -> Result<(), CaptureError> {
            if !self.capturing {
                return Err(CaptureError::NotCapturing);
            }

            // Signal the capture handler to stop
            if let Some(flags) = &self.flags {
                flags.should_stop.store(true, Ordering::Relaxed);
            }

            // Stop the capture control (waits for thread)
            if let Some(control) = self.control.take() {
                let _ = control.stop();
            }

            self.flags = None;
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
