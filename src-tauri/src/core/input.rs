//! Input monitoring abstraction trait.
//! Captures mouse position, clicks, keyboard events, scroll, and drag events.

use serde::{Deserialize, Serialize};

use super::coordinates::NormalizedPoint;

/// Mouse position sample (60Hz)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MousePositionSample {
    /// Seconds since recording start
    pub time: f64,
    /// Normalized position (0-1, top-left origin)
    pub position: NormalizedPoint,
    /// Velocity in normalized units per second
    pub velocity: f64,
}

/// Mouse click event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MouseClickRecord {
    pub time: f64,
    pub position: NormalizedPoint,
    pub button: MouseButton,
    pub duration: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

/// Keyboard event record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyboardRecord {
    pub time: f64,
    pub event_type: KeyAction,
    pub key_code: u16,
    pub character: Option<String>,
    pub modifiers: ModifierState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum KeyAction {
    Down,
    Up,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct ModifierState {
    pub command: bool,
    pub shift: bool,
    pub alt: bool,
    pub control: bool,
}

/// Scroll event record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScrollRecord {
    pub time: f64,
    pub position: NormalizedPoint,
    pub delta_x: f64,
    pub delta_y: f64,
    pub is_trackpad: bool,
}

/// Drag event record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DragRecord {
    pub start_time: f64,
    pub end_time: f64,
    pub start_position: NormalizedPoint,
    pub end_position: NormalizedPoint,
}

/// Complete input recording data
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InputRecording {
    pub positions: Vec<MousePositionSample>,
    pub clicks: Vec<MouseClickRecord>,
    pub keyboard: Vec<KeyboardRecord>,
    pub scrolls: Vec<ScrollRecord>,
    pub drags: Vec<DragRecord>,
}

impl InputRecording {
    pub fn new() -> Self {
        Self::default()
    }

    /// Serialize to JSON for saving alongside video
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Deserialize from JSON
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

/// Input monitoring error types
#[derive(Debug, thiserror::Error)]
pub enum InputError {
    #[error("Already monitoring")]
    AlreadyMonitoring,
    #[error("Not monitoring")]
    NotMonitoring,
    #[error("Permission denied for input monitoring")]
    PermissionDenied,
    #[error("Platform error: {0}")]
    Platform(String),
}

/// Input monitor abstraction trait.
/// Each platform provides its own implementation.
pub trait InputMonitor: Send {
    /// Start monitoring all input events.
    /// Events are collected internally and retrieved via stop_monitoring().
    fn start_monitoring(&mut self) -> Result<(), InputError>;

    /// Stop monitoring and return all collected input data.
    fn stop_monitoring(&mut self) -> Result<InputRecording, InputError>;

    /// Check if currently monitoring
    fn is_monitoring(&self) -> bool;
}

/// Stub implementation for development/testing
pub struct StubInputMonitor {
    monitoring: bool,
}

impl StubInputMonitor {
    pub fn new() -> Self {
        Self { monitoring: false }
    }
}

impl InputMonitor for StubInputMonitor {
    fn start_monitoring(&mut self) -> Result<(), InputError> {
        if self.monitoring {
            return Err(InputError::AlreadyMonitoring);
        }
        self.monitoring = true;
        Ok(())
    }

    fn stop_monitoring(&mut self) -> Result<InputRecording, InputError> {
        if !self.monitoring {
            return Err(InputError::NotMonitoring);
        }
        self.monitoring = false;
        Ok(InputRecording::new())
    }

    fn is_monitoring(&self) -> bool {
        self.monitoring
    }
}

/// Platform-specific input monitor implementations
#[cfg(target_os = "windows")]
pub mod windows {
    use super::*;

    pub struct WindowsInputMonitor {
        monitoring: bool,
        recording: InputRecording,
    }

    impl WindowsInputMonitor {
        pub fn new() -> Self {
            Self {
                monitoring: false,
                recording: InputRecording::new(),
            }
        }
    }

    impl InputMonitor for WindowsInputMonitor {
        fn start_monitoring(&mut self) -> Result<(), InputError> {
            if self.monitoring {
                return Err(InputError::AlreadyMonitoring);
            }
            self.recording = InputRecording::new();
            // TODO: SetWindowsHookEx for WH_MOUSE_LL and WH_KEYBOARD_LL
            // TODO: Start 60Hz position polling timer (GetCursorPos)
            self.monitoring = true;
            Ok(())
        }

        fn stop_monitoring(&mut self) -> Result<InputRecording, InputError> {
            if !self.monitoring {
                return Err(InputError::NotMonitoring);
            }
            // TODO: UnhookWindowsHookEx, stop timer
            self.monitoring = false;
            Ok(std::mem::take(&mut self.recording))
        }

        fn is_monitoring(&self) -> bool {
            self.monitoring
        }
    }
}

#[cfg(target_os = "linux")]
pub mod linux {
    use super::*;

    pub struct LinuxInputMonitor {
        monitoring: bool,
        recording: InputRecording,
    }

    impl LinuxInputMonitor {
        pub fn new() -> Self {
            Self {
                monitoring: false,
                recording: InputRecording::new(),
            }
        }
    }

    impl InputMonitor for LinuxInputMonitor {
        fn start_monitoring(&mut self) -> Result<(), InputError> {
            if self.monitoring {
                return Err(InputError::AlreadyMonitoring);
            }
            self.recording = InputRecording::new();
            // TODO: X11 XRecord extension or libinput for event capture
            // TODO: Start 60Hz position polling (XQueryPointer)
            self.monitoring = true;
            Ok(())
        }

        fn stop_monitoring(&mut self) -> Result<InputRecording, InputError> {
            if !self.monitoring {
                return Err(InputError::NotMonitoring);
            }
            self.monitoring = false;
            Ok(std::mem::take(&mut self.recording))
        }

        fn is_monitoring(&self) -> bool {
            self.monitoring
        }
    }
}

/// Create the platform-appropriate input monitor
pub fn create_input_monitor() -> Box<dyn InputMonitor> {
    #[cfg(target_os = "windows")]
    { Box::new(windows::WindowsInputMonitor::new()) }

    #[cfg(target_os = "linux")]
    { Box::new(linux::LinuxInputMonitor::new()) }

    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    { Box::new(StubInputMonitor::new()) }
}
