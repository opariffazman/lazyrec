//! Input monitoring abstraction trait.
//! Captures mouse position, clicks, keyboard events, scroll, and drag events.

use serde::{Deserialize, Serialize};

use super::coordinates::NormalizedPoint;

/// Mouse position sample (60Hz, matches video frame rate)
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

/// Windows input monitoring via low-level hooks (SetWindowsHookEx) and GetCursorPos polling.
///
/// Architecture:
/// - A dedicated thread runs the Windows message loop for hook callbacks
/// - A 60Hz timer thread polls GetCursorPos for mouse position samples
/// - All events are collected into a shared InputRecording
/// - stop_monitoring() signals threads to stop and returns the recording
#[cfg(target_os = "windows")]
pub mod windows {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex};
    use std::time::Instant;

    use ::windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
    use ::windows::Win32::Graphics::Gdi::{GetDC, GetDeviceCaps, ReleaseDC, HORZRES, VERTRES};
    use ::windows::Win32::UI::WindowsAndMessaging::{
        CallNextHookEx, GetCursorPos, SetWindowsHookExW, UnhookWindowsHookEx,
        GetMessageW, PeekMessageW, HHOOK, KBDLLHOOKSTRUCT, MSLLHOOKSTRUCT, MSG,
        PM_NOREMOVE, WH_KEYBOARD_LL, WH_MOUSE_LL, WM_KEYDOWN, WM_KEYUP, WM_LBUTTONDOWN,
        WM_LBUTTONUP, WM_MBUTTONDOWN, WM_MBUTTONUP, WM_MOUSEWHEEL,
        WM_RBUTTONDOWN, WM_RBUTTONUP, WM_SYSKEYDOWN, WM_SYSKEYUP,
    };
    use std::sync::atomic::AtomicU32;

    /// Thread-safe shared state for hook callbacks
    struct HookState {
        recording: Mutex<InputRecording>,
        start_time: Instant,
        should_stop: AtomicBool,
        screen_width: f64,
        screen_height: f64,
        last_position: Mutex<NormalizedPoint>,
        /// Thread ID of the hook thread (for PostThreadMessageW on stop)
        hook_thread_id: AtomicU32,
    }

    // Global state for hook callbacks (Windows hooks require static/global access).
    // Uses Mutex<Option<>> instead of OnceLock so it can be reset between recordings.
    static HOOK_STATE: std::sync::Mutex<Option<Arc<HookState>>> = std::sync::Mutex::new(None);

    fn elapsed(state: &HookState) -> f64 {
        state.start_time.elapsed().as_secs_f64()
    }

    fn normalize_point(state: &HookState, x: i32, y: i32) -> NormalizedPoint {
        NormalizedPoint::new(
            (x as f64 / state.screen_width).clamp(0.0, 1.0),
            (y as f64 / state.screen_height).clamp(0.0, 1.0),
        )
    }

    unsafe extern "system" fn mouse_hook_proc(
        n_code: i32,
        w_param: WPARAM,
        l_param: LPARAM,
    ) -> LRESULT {
        if n_code >= 0 {
            if let Some(state) = HOOK_STATE.lock().unwrap().as_ref().cloned() {
                let info = &*(l_param.0 as *const MSLLHOOKSTRUCT);
                let pos = normalize_point(&state, info.pt.x, info.pt.y);
                let time = elapsed(&state);

                let msg = w_param.0 as u32;
                match msg {
                    WM_LBUTTONDOWN | WM_RBUTTONDOWN | WM_MBUTTONDOWN => {
                        let button = match msg {
                            WM_LBUTTONDOWN => MouseButton::Left,
                            WM_RBUTTONDOWN => MouseButton::Right,
                            _ => MouseButton::Middle,
                        };
                        if let Ok(mut rec) = state.recording.lock() {
                            rec.clicks.push(MouseClickRecord {
                                time,
                                position: pos,
                                button,
                                duration: 0.0, // Updated on button up
                            });
                        }
                    }
                    WM_LBUTTONUP | WM_RBUTTONUP | WM_MBUTTONUP => {
                        // Update duration of last matching click
                        let button = match msg {
                            WM_LBUTTONUP => MouseButton::Left,
                            WM_RBUTTONUP => MouseButton::Right,
                            _ => MouseButton::Middle,
                        };
                        if let Ok(mut rec) = state.recording.lock() {
                            if let Some(click) = rec.clicks.iter_mut().rev()
                                .find(|c| c.button == button && c.duration == 0.0)
                            {
                                click.duration = time - click.time;
                            }
                        }
                    }
                    WM_MOUSEWHEEL => {
                        let delta = (info.mouseData >> 16) as i16 as f64 / 120.0;
                        if let Ok(mut rec) = state.recording.lock() {
                            rec.scrolls.push(ScrollRecord {
                                time,
                                position: pos,
                                delta_x: 0.0,
                                delta_y: delta,
                                is_trackpad: false,
                            });
                        }
                    }
                    _ => {}
                }
            }
        }
        unsafe { CallNextHookEx(HHOOK::default(), n_code, w_param, l_param) }
    }

    unsafe extern "system" fn keyboard_hook_proc(
        n_code: i32,
        w_param: WPARAM,
        l_param: LPARAM,
    ) -> LRESULT {
        if n_code >= 0 {
            if let Some(state) = HOOK_STATE.lock().unwrap().as_ref().cloned() {
                let info = &*(l_param.0 as *const KBDLLHOOKSTRUCT);
                let time = elapsed(&state);
                let msg = w_param.0 as u32;

                let event_type = match msg {
                    WM_KEYDOWN | WM_SYSKEYDOWN => KeyAction::Down,
                    WM_KEYUP | WM_SYSKEYUP => KeyAction::Up,
                    _ => KeyAction::Down,
                };

                // Read modifier state from flags
                let modifiers = ModifierState {
                    shift: (unsafe { ::windows::Win32::UI::Input::KeyboardAndMouse::GetKeyState(0x10) } & 0x8000u16 as i16) != 0,
                    control: (unsafe { ::windows::Win32::UI::Input::KeyboardAndMouse::GetKeyState(0x11) } & 0x8000u16 as i16) != 0,
                    alt: (unsafe { ::windows::Win32::UI::Input::KeyboardAndMouse::GetKeyState(0x12) } & 0x8000u16 as i16) != 0,
                    command: (unsafe { ::windows::Win32::UI::Input::KeyboardAndMouse::GetKeyState(0x5B) } & 0x8000u16 as i16) != 0,
                };

                if let Ok(mut rec) = state.recording.lock() {
                    rec.keyboard.push(KeyboardRecord {
                        time,
                        event_type,
                        key_code: info.vkCode as u16,
                        character: None, // VK to char mapping can be added later
                        modifiers,
                    });
                }
            }
        }
        unsafe { CallNextHookEx(HHOOK::default(), n_code, w_param, l_param) }
    }

    pub struct WindowsInputMonitor {
        monitoring: bool,
        hook_thread: Option<std::thread::JoinHandle<()>>,
        poll_thread: Option<std::thread::JoinHandle<()>>,
    }

    impl WindowsInputMonitor {
        pub fn new() -> Self {
            Self {
                monitoring: false,
                hook_thread: None,
                poll_thread: None,
            }
        }

        fn get_screen_size() -> (f64, f64) {
            unsafe {
                let hdc = GetDC(None);
                let w = GetDeviceCaps(hdc, HORZRES) as f64;
                let h = GetDeviceCaps(hdc, VERTRES) as f64;
                let _ = ReleaseDC(None, hdc);
                (if w > 0.0 { w } else { 1920.0 }, if h > 0.0 { h } else { 1080.0 })
            }
        }
    }

    impl InputMonitor for WindowsInputMonitor {
        fn start_monitoring(&mut self) -> Result<(), InputError> {
            if self.monitoring {
                return Err(InputError::AlreadyMonitoring);
            }

            let (sw, sh) = Self::get_screen_size();

            let state = Arc::new(HookState {
                recording: Mutex::new(InputRecording::new()),
                start_time: Instant::now(),
                should_stop: AtomicBool::new(false),
                screen_width: sw,
                screen_height: sh,
                last_position: Mutex::new(NormalizedPoint::CENTER),
                hook_thread_id: AtomicU32::new(0),
            });

            // Store in global for hook callbacks
            *HOOK_STATE.lock().unwrap() = Some(state.clone());

            // Hook thread: installs low-level hooks and runs message loop
            let state_hook = state.clone();
            self.hook_thread = Some(std::thread::spawn(move || {
                unsafe {
                    // Store this thread's ID so stop_monitoring can post WM_QUIT to it
                    let tid = ::windows::Win32::System::Threading::GetCurrentThreadId();
                    state_hook.hook_thread_id.store(tid, Ordering::Relaxed);

                    let mouse_hook = SetWindowsHookExW(
                        WH_MOUSE_LL,
                        Some(mouse_hook_proc),
                        None,
                        0,
                    );

                    let kb_hook = SetWindowsHookExW(
                        WH_KEYBOARD_LL,
                        Some(keyboard_hook_proc),
                        None,
                        0,
                    );

                    // Non-blocking message loop: pump messages but check should_stop regularly
                    let mut msg = MSG::default();
                    while !state_hook.should_stop.load(Ordering::Relaxed) {
                        // PeekMessageW is non-blocking — returns immediately if no messages
                        while PeekMessageW(&mut msg, None, 0, 0, PM_NOREMOVE).as_bool() {
                            if GetMessageW(&mut msg, None, 0, 0).0 <= 0 {
                                // WM_QUIT received
                                state_hook.should_stop.store(true, Ordering::Relaxed);
                                break;
                            }
                        }
                        if state_hook.should_stop.load(Ordering::Relaxed) {
                            break;
                        }
                        std::thread::sleep(std::time::Duration::from_millis(10));
                    }

                    if let Ok(h) = mouse_hook {
                        let _ = UnhookWindowsHookEx(h);
                    }
                    if let Ok(h) = kb_hook {
                        let _ = UnhookWindowsHookEx(h);
                    }
                }
            }));

            // Poll thread: 60Hz mouse position sampling
            // Matches video frame rate for 1:1 alignment (Screenize pattern).
            // Cursor smoothing during export uses interpolation, not higher sample rate.
            let state_poll = state.clone();
            self.poll_thread = Some(std::thread::spawn(move || {
                let interval = std::time::Duration::from_micros(16_667); // ~60Hz
                while !state_poll.should_stop.load(Ordering::Relaxed) {
                    let time = elapsed(&state_poll);

                    let mut point = ::windows::Win32::Foundation::POINT { x: 0, y: 0 };
                    unsafe { let _ = GetCursorPos(&mut point); }

                    let pos = normalize_point(&state_poll, point.x, point.y);

                    // Compute velocity from last position
                    let velocity = {
                        let mut last = state_poll.last_position.lock().unwrap();
                        let v = last.distance(&pos) * 60.0; // per second (matches poll rate)
                        *last = pos;
                        v
                    };

                    if let Ok(mut rec) = state_poll.recording.lock() {
                        rec.positions.push(MousePositionSample {
                            time,
                            position: pos,
                            velocity,
                        });
                    }

                    std::thread::sleep(interval);
                }
            }));

            self.monitoring = true;
            Ok(())
        }

        fn stop_monitoring(&mut self) -> Result<InputRecording, InputError> {
            if !self.monitoring {
                return Err(InputError::NotMonitoring);
            }

            log::info!("Input monitor: signaling threads to stop...");

            // Signal threads to stop
            if let Some(state) = HOOK_STATE.lock().unwrap().as_ref().cloned() {
                state.should_stop.store(true, Ordering::Relaxed);

                // Post WM_QUIT to the hook thread's message queue using its actual thread ID
                let tid = state.hook_thread_id.load(Ordering::Relaxed);
                if tid != 0 {
                    log::info!("Input monitor: posting WM_QUIT to hook thread (tid={tid})");
                    unsafe {
                        let _ = ::windows::Win32::UI::WindowsAndMessaging::PostThreadMessageW(
                            tid, 0x0012 /* WM_QUIT */, WPARAM(0), LPARAM(0),
                        );
                    }
                }
            }

            // Wait for poll thread (should exit quickly from sleep loop)
            if let Some(h) = self.poll_thread.take() {
                let _ = h.join();
            }
            log::info!("Input monitor: poll thread stopped");

            // Wait for hook thread with timeout
            if let Some(h) = self.hook_thread.take() {
                let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
                let mut joined = false;
                while std::time::Instant::now() < deadline {
                    if h.is_finished() {
                        let _ = h.join();
                        joined = true;
                        break;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(50));
                }
                if !joined {
                    log::warn!("Input monitor: hook thread did not stop within 2s, abandoning");
                } else {
                    log::info!("Input monitor: hook thread stopped");
                }
            }

            // Extract recording and clear global state for next recording
            let recording = {
                let mut global = HOOK_STATE.lock().unwrap();
                let rec = if let Some(state) = global.as_ref() {
                    std::mem::take(&mut *state.recording.lock().unwrap())
                } else {
                    InputRecording::new()
                };
                *global = None; // Clear so next recording can set fresh state
                rec
            };

            self.monitoring = false;
            log::info!("Input monitor: stopped, collected {} positions, {} clicks, {} keystrokes",
                recording.positions.len(), recording.clicks.len(), recording.keyboard.len());
            Ok(recording)
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

#[cfg(test)]
mod tests {
    use super::*;

    // InputRecording JSON roundtrip

    #[test]
    fn test_input_recording_empty_roundtrip() {
        let rec = InputRecording::new();
        let json = rec.to_json().unwrap();
        let restored = InputRecording::from_json(&json).unwrap();
        assert!(restored.positions.is_empty());
        assert!(restored.clicks.is_empty());
        assert!(restored.keyboard.is_empty());
        assert!(restored.scrolls.is_empty());
        assert!(restored.drags.is_empty());
    }

    #[test]
    fn test_input_recording_with_data_roundtrip() {
        let mut rec = InputRecording::new();
        rec.positions.push(MousePositionSample {
            time: 0.5,
            position: NormalizedPoint::new(0.3, 0.7),
            velocity: 1.2,
        });
        rec.clicks.push(MouseClickRecord {
            time: 1.0,
            position: NormalizedPoint::CENTER,
            button: MouseButton::Left,
            duration: 0.1,
        });
        rec.keyboard.push(KeyboardRecord {
            time: 2.0,
            event_type: KeyAction::Down,
            key_code: 65,
            character: Some("a".into()),
            modifiers: ModifierState { command: false, shift: true, alt: false, control: false },
        });
        rec.scrolls.push(ScrollRecord {
            time: 3.0,
            position: NormalizedPoint::new(0.5, 0.5),
            delta_x: 0.0,
            delta_y: -3.0,
            is_trackpad: true,
        });
        rec.drags.push(DragRecord {
            start_time: 4.0,
            end_time: 5.0,
            start_position: NormalizedPoint::new(0.1, 0.1),
            end_position: NormalizedPoint::new(0.9, 0.9),
        });
        let json = rec.to_json().unwrap();
        let restored = InputRecording::from_json(&json).unwrap();
        assert_eq!(restored.positions.len(), 1);
        assert_eq!(restored.clicks.len(), 1);
        assert_eq!(restored.keyboard.len(), 1);
        assert_eq!(restored.scrolls.len(), 1);
        assert_eq!(restored.drags.len(), 1);
    }

    #[test]
    fn test_input_recording_from_invalid_json() {
        assert!(InputRecording::from_json("not json").is_err());
    }

    // StubInputMonitor state machine

    #[test]
    fn test_stub_monitor_initial_state() {
        let monitor = StubInputMonitor::new();
        assert!(!monitor.is_monitoring());
    }

    #[test]
    fn test_stub_monitor_start_stop() {
        let mut monitor = StubInputMonitor::new();
        assert!(monitor.start_monitoring().is_ok());
        assert!(monitor.is_monitoring());
        let recording = monitor.stop_monitoring().unwrap();
        assert!(!monitor.is_monitoring());
        assert!(recording.positions.is_empty());
    }

    #[test]
    fn test_stub_monitor_double_start_errors() {
        let mut monitor = StubInputMonitor::new();
        monitor.start_monitoring().unwrap();
        match monitor.start_monitoring() {
            Err(InputError::AlreadyMonitoring) => {}
            other => panic!("Expected AlreadyMonitoring, got {:?}", other),
        }
    }

    #[test]
    fn test_stub_monitor_stop_without_start_errors() {
        let mut monitor = StubInputMonitor::new();
        match monitor.stop_monitoring() {
            Err(InputError::NotMonitoring) => {}
            other => panic!("Expected NotMonitoring, got {:?}", other),
        }
    }

    // MouseButton serde

    #[test]
    fn test_mouse_button_serde() {
        for btn in [MouseButton::Left, MouseButton::Right, MouseButton::Middle] {
            let json = serde_json::to_string(&btn).unwrap();
            let restored: MouseButton = serde_json::from_str(&json).unwrap();
            assert_eq!(btn, restored);
        }
    }

    // KeyAction serde

    #[test]
    fn test_key_action_serde() {
        for action in [KeyAction::Down, KeyAction::Up] {
            let json = serde_json::to_string(&action).unwrap();
            let restored: KeyAction = serde_json::from_str(&json).unwrap();
            assert_eq!(action, restored);
        }
    }

    // ModifierState default

    #[test]
    fn test_modifier_state_default() {
        let m = ModifierState::default();
        assert!(!m.command && !m.shift && !m.alt && !m.control);
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
