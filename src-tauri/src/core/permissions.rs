//! Platform permissions manager.
//! Checks and requests permissions for screen capture, input monitoring, etc.

use serde::{Deserialize, Serialize};

/// Permission status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PermissionStatus {
    /// Permission granted
    Granted,
    /// Permission denied by user
    Denied,
    /// Permission not yet requested
    NotDetermined,
    /// Permission not applicable on this platform
    NotApplicable,
}

/// Permission types needed by LazyRec
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PermissionType {
    ScreenCapture,
    InputMonitoring,
    Accessibility,
    AudioCapture,
}

impl PermissionType {
    pub fn display_name(&self) -> &str {
        match self {
            Self::ScreenCapture => "Screen Capture",
            Self::InputMonitoring => "Input Monitoring",
            Self::Accessibility => "Accessibility",
            Self::AudioCapture => "Audio Capture",
        }
    }

    pub fn description(&self) -> &str {
        match self {
            Self::ScreenCapture => "Required to record your screen or window",
            Self::InputMonitoring => "Required to track mouse and keyboard events",
            Self::Accessibility => "Required to detect UI elements for smart zoom",
            Self::AudioCapture => "Required to record system or microphone audio",
        }
    }
}

/// Result of checking all permissions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionReport {
    pub screen_capture: PermissionStatus,
    pub input_monitoring: PermissionStatus,
    pub accessibility: PermissionStatus,
    pub audio_capture: PermissionStatus,
}

impl PermissionReport {
    /// Check if all required permissions for recording are granted
    pub fn can_record(&self) -> bool {
        (self.screen_capture == PermissionStatus::Granted
            || self.screen_capture == PermissionStatus::NotApplicable)
            && (self.input_monitoring == PermissionStatus::Granted
                || self.input_monitoring == PermissionStatus::NotApplicable)
    }

    /// List permissions that need attention
    pub fn missing_permissions(&self) -> Vec<PermissionType> {
        let mut missing = Vec::new();
        if self.screen_capture == PermissionStatus::Denied
            || self.screen_capture == PermissionStatus::NotDetermined
        {
            missing.push(PermissionType::ScreenCapture);
        }
        if self.input_monitoring == PermissionStatus::Denied
            || self.input_monitoring == PermissionStatus::NotDetermined
        {
            missing.push(PermissionType::InputMonitoring);
        }
        missing
    }
}

/// Permissions manager trait
pub trait PermissionsManager: Send {
    /// Check all permission statuses
    fn check_all(&self) -> PermissionReport;

    /// Check a specific permission
    fn check(&self, permission: PermissionType) -> PermissionStatus;

    /// Request a specific permission (may show OS dialog)
    fn request(&self, permission: PermissionType) -> PermissionStatus;
}

/// Windows permissions manager.
/// Most permissions are available by default on Windows.
pub struct PlatformPermissions;

impl PlatformPermissions {
    pub fn new() -> Self {
        Self
    }
}

impl PermissionsManager for PlatformPermissions {
    fn check_all(&self) -> PermissionReport {
        PermissionReport {
            screen_capture: self.check(PermissionType::ScreenCapture),
            input_monitoring: self.check(PermissionType::InputMonitoring),
            accessibility: self.check(PermissionType::Accessibility),
            audio_capture: self.check(PermissionType::AudioCapture),
        }
    }

    fn check(&self, permission: PermissionType) -> PermissionStatus {
        match permission {
            #[cfg(target_os = "windows")]
            PermissionType::ScreenCapture => {
                // Windows: DXGI Desktop Duplication available to all desktop apps
                // WinRT GraphicsCapture may require manifest capabilities
                PermissionStatus::Granted
            }
            #[cfg(target_os = "windows")]
            PermissionType::InputMonitoring => {
                // Windows: Low-level hooks available to desktop apps
                PermissionStatus::Granted
            }
            #[cfg(target_os = "windows")]
            PermissionType::Accessibility => {
                // Windows: UI Automation always available
                PermissionStatus::Granted
            }
            #[cfg(target_os = "windows")]
            PermissionType::AudioCapture => {
                // Windows: WASAPI available to desktop apps
                PermissionStatus::Granted
            }

            #[cfg(target_os = "linux")]
            PermissionType::ScreenCapture => {
                // Linux: PipeWire portal requires user consent per-session
                // TODO: Check if PipeWire is available
                PermissionStatus::NotDetermined
            }
            #[cfg(target_os = "linux")]
            PermissionType::InputMonitoring => {
                // Linux: Requires user to be in 'input' group for evdev
                // X11 XRecord may work without special permissions
                // TODO: Check group membership
                PermissionStatus::NotDetermined
            }
            #[cfg(target_os = "linux")]
            PermissionType::Accessibility => {
                // Linux: AT-SPI2 needs to be enabled
                // TODO: Check AT-SPI2 availability
                PermissionStatus::NotDetermined
            }
            #[cfg(target_os = "linux")]
            PermissionType::AudioCapture => {
                // Linux: PulseAudio/PipeWire usually available
                PermissionStatus::NotDetermined
            }

            #[cfg(not(any(target_os = "windows", target_os = "linux")))]
            _ => PermissionStatus::NotApplicable,
        }
    }

    fn request(&self, permission: PermissionType) -> PermissionStatus {
        // On most platforms, requesting just re-checks status.
        // On Linux with PipeWire, this would trigger the portal dialog.
        self.check(permission)
    }
}

/// Create the platform-appropriate permissions manager
pub fn create_permissions_manager() -> Box<dyn PermissionsManager> {
    Box::new(PlatformPermissions::new())
}
