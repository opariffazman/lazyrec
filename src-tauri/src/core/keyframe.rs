use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::coordinates::NormalizedPoint;
use super::easing::EasingCurve;

// MARK: - Transform Keyframe

/// Transform (zoom/pan) keyframe
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TransformKeyframe {
    pub id: Uuid,
    /// Time in seconds
    pub time: f64,
    /// 1.0 = 100%, 2.0 = 200%
    pub zoom: f64,
    /// Normalized center position (0-1, top-left origin)
    pub center: NormalizedPoint,
    /// Interpolation mode to the next keyframe
    pub easing: EasingCurve,
}

impl TransformKeyframe {
    pub fn new(time: f64, zoom: f64, center: NormalizedPoint, easing: EasingCurve) -> Self {
        Self {
            id: Uuid::new_v4(),
            time,
            zoom: zoom.max(1.0),
            center: center.clamped(),
            easing,
        }
    }

    /// Identity keyframe (no zoom, centered)
    pub fn identity(time: f64) -> Self {
        Self::new(time, 1.0, NormalizedPoint::CENTER, EasingCurve::spring_default())
    }

    pub fn value(&self) -> TransformValue {
        TransformValue {
            zoom: self.zoom,
            center: self.center,
        }
    }
}

/// Transform value (for interpolation)
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TransformValue {
    pub zoom: f64,
    pub center: NormalizedPoint,
}

impl TransformValue {
    pub const IDENTITY: Self = Self {
        zoom: 1.0,
        center: NormalizedPoint::CENTER,
    };

    pub fn interpolated(&self, to: &Self, amount: f64) -> Self {
        Self {
            zoom: self.zoom + (to.zoom - self.zoom) * amount,
            center: NormalizedPoint::new(
                self.center.x + (to.center.x - self.center.x) * amount,
                self.center.y + (to.center.y - self.center.y) * amount,
            ),
        }
    }

    /// Interpolation tuned for window mode.
    /// Linearly interpolates the anchor point (center * zoom) so the visual
    /// position changes at the same rate as zoom.
    pub fn interpolated_for_window_mode(&self, to: &Self, amount: f64) -> Self {
        let interpolated_zoom = self.zoom + (to.zoom - self.zoom) * amount;

        let start_anchor_x = self.center.x * self.zoom;
        let start_anchor_y = self.center.y * self.zoom;
        let end_anchor_x = to.center.x * to.zoom;
        let end_anchor_y = to.center.y * to.zoom;

        let interp_anchor_x = start_anchor_x + (end_anchor_x - start_anchor_x) * amount;
        let interp_anchor_y = start_anchor_y + (end_anchor_y - start_anchor_y) * amount;

        let safe_zoom = interpolated_zoom.max(0.001);
        Self {
            zoom: interpolated_zoom,
            center: NormalizedPoint::new(interp_anchor_x / safe_zoom, interp_anchor_y / safe_zoom),
        }
    }
}

// MARK: - Ripple Keyframe

/// Ripple colors
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum RippleColor {
    #[serde(rename = "leftClick")]
    LeftClick,
    #[serde(rename = "rightClick")]
    RightClick,
    #[serde(rename = "custom")]
    Custom { r: f64, g: f64, b: f64, a: f64 },
}

impl RippleColor {
    pub fn rgba(&self) -> (f64, f64, f64, f64) {
        match self {
            Self::LeftClick => (0.2, 0.5, 1.0, 0.6),
            Self::RightClick => (1.0, 0.5, 0.2, 0.6),
            Self::Custom { r, g, b, a } => (*r, *g, *b, *a),
        }
    }
}

/// Ripple effect keyframe
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RippleKeyframe {
    pub id: Uuid,
    /// Time in seconds
    pub time: f64,
    /// Normalized position (0-1, top-left origin)
    pub position: NormalizedPoint,
    /// 0.0-1.0
    pub intensity: f64,
    /// Duration of the ripple animation
    pub duration: f64,
    pub color: RippleColor,
    pub easing: EasingCurve,
}

impl RippleKeyframe {
    pub fn new(time: f64, position: NormalizedPoint) -> Self {
        Self {
            id: Uuid::new_v4(),
            time,
            position: position.clamped(),
            intensity: 0.8,
            duration: 0.4,
            color: RippleColor::LeftClick,
            easing: EasingCurve::spring_bouncy(),
        }
    }

    pub fn end_time(&self) -> f64 {
        self.time + self.duration
    }

    pub fn is_active(&self, current_time: f64) -> bool {
        current_time >= self.time && current_time <= self.end_time()
    }

    pub fn progress(&self, current_time: f64) -> f64 {
        if !self.is_active(current_time) || self.duration <= 0.0 {
            return 0.0;
        }
        let elapsed = current_time - self.time;
        elapsed / self.duration
    }
}

// MARK: - Cursor Style Keyframe

/// Cursor styles
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CursorStyle {
    Arrow,
    Pointer,
    IBeam,
    Crosshair,
    OpenHand,
    ClosedHand,
    ContextMenu,
}

impl CursorStyle {
    pub fn display_name(&self) -> &str {
        match self {
            Self::Arrow => "Arrow",
            Self::Pointer => "Pointer",
            Self::IBeam => "I-Beam",
            Self::Crosshair => "Crosshair",
            Self::OpenHand => "Open Hand",
            Self::ClosedHand => "Closed Hand",
            Self::ContextMenu => "Context Menu",
        }
    }
}

/// Cursor style keyframe
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CursorStyleKeyframe {
    pub id: Uuid,
    pub time: f64,
    /// nil uses the original mouse data position
    pub position: Option<NormalizedPoint>,
    pub style: CursorStyle,
    pub visible: bool,
    pub scale: f64,
    /// Velocity (used for motion blur intensity, normalized per second)
    pub velocity: Option<f64>,
    /// Movement direction (radians, for motion blur)
    pub movement_direction: Option<f64>,
    pub easing: EasingCurve,
}

impl CursorStyleKeyframe {
    pub fn new(time: f64) -> Self {
        Self {
            id: Uuid::new_v4(),
            time,
            position: None,
            style: CursorStyle::Arrow,
            visible: true,
            scale: 2.5,
            velocity: None,
            movement_direction: None,
            easing: EasingCurve::spring_snappy(),
        }
    }
}

// MARK: - Keystroke Keyframe

/// Keystroke overlay keyframe
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KeystrokeKeyframe {
    pub id: Uuid,
    /// Keystroke start time
    pub time: f64,
    /// Display text (e.g., "Ctrl+C", "Shift+Ctrl+Z")
    #[serde(rename = "displayText")]
    pub display_text: String,
    /// Overlay display duration
    pub duration: f64,
    /// Fade-in duration
    #[serde(rename = "fadeInDuration")]
    pub fade_in_duration: f64,
    /// Fade-out duration
    #[serde(rename = "fadeOutDuration")]
    pub fade_out_duration: f64,
    /// Overlay center position (default: bottom-center)
    pub position: NormalizedPoint,
    pub easing: EasingCurve,
}

impl KeystrokeKeyframe {
    pub fn new(time: f64, display_text: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            time,
            display_text,
            duration: 1.5,
            fade_in_duration: 0.15,
            fade_out_duration: 0.3,
            position: NormalizedPoint::new(0.5, 0.95),
            easing: EasingCurve::EaseOut,
        }
    }

    pub fn end_time(&self) -> f64 {
        self.time + self.duration
    }

    pub fn is_active(&self, current_time: f64) -> bool {
        current_time >= self.time && current_time <= self.end_time()
    }

    pub fn progress(&self, current_time: f64) -> f64 {
        if !self.is_active(current_time) || self.duration <= 0.0 {
            return 0.0;
        }
        (current_time - self.time) / self.duration
    }

    /// Opacity at the given time (with fade-in/out applied)
    pub fn opacity(&self, current_time: f64) -> f64 {
        if !self.is_active(current_time) {
            return 0.0;
        }
        let elapsed = current_time - self.time;
        let remaining = self.end_time() - current_time;

        if elapsed < self.fade_in_duration && self.fade_in_duration > 0.0 {
            return elapsed / self.fade_in_duration;
        }
        if remaining < self.fade_out_duration && self.fade_out_duration > 0.0 {
            return remaining / self.fade_out_duration;
        }
        1.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // TransformKeyframe tests

    #[test]
    fn test_transform_keyframe_new_clamps_zoom() {
        let kf = TransformKeyframe::new(0.0, 0.5, NormalizedPoint::CENTER, EasingCurve::Linear);
        assert_eq!(kf.zoom, 1.0); // min zoom is 1.0
    }

    #[test]
    fn test_transform_keyframe_new_clamps_center() {
        let kf = TransformKeyframe::new(0.0, 2.0, NormalizedPoint::new(-0.5, 1.5), EasingCurve::Linear);
        assert_eq!(kf.center.x, 0.0);
        assert_eq!(kf.center.y, 1.0);
    }

    #[test]
    fn test_transform_keyframe_identity() {
        let kf = TransformKeyframe::identity(5.0);
        assert_eq!(kf.time, 5.0);
        assert_eq!(kf.zoom, 1.0);
        assert_eq!(kf.center, NormalizedPoint::CENTER);
    }

    #[test]
    fn test_transform_value_identity() {
        let v = TransformValue::IDENTITY;
        assert_eq!(v.zoom, 1.0);
        assert_eq!(v.center, NormalizedPoint::CENTER);
    }

    #[test]
    fn test_transform_value_interpolated() {
        let a = TransformValue { zoom: 1.0, center: NormalizedPoint::new(0.0, 0.0) };
        let b = TransformValue { zoom: 3.0, center: NormalizedPoint::new(1.0, 1.0) };
        let mid = a.interpolated(&b, 0.5);
        assert!((mid.zoom - 2.0).abs() < 1e-9);
        assert!((mid.center.x - 0.5).abs() < 1e-9);
        assert!((mid.center.y - 0.5).abs() < 1e-9);
    }

    #[test]
    fn test_transform_value_interpolated_window_mode() {
        let a = TransformValue { zoom: 1.0, center: NormalizedPoint::new(0.5, 0.5) };
        let b = TransformValue { zoom: 2.0, center: NormalizedPoint::new(0.5, 0.5) };
        let mid = a.interpolated_for_window_mode(&b, 0.5);
        assert!((mid.zoom - 1.5).abs() < 1e-9);
        // center should stay roughly 0.5 when center is same
        assert!((mid.center.x - 0.5).abs() < 0.01);
    }

    // RippleKeyframe tests

    #[test]
    fn test_ripple_color_rgba() {
        assert_eq!(RippleColor::LeftClick.rgba(), (0.2, 0.5, 1.0, 0.6));
        assert_eq!(RippleColor::RightClick.rgba(), (1.0, 0.5, 0.2, 0.6));
        let custom = RippleColor::Custom { r: 0.1, g: 0.2, b: 0.3, a: 0.4 };
        assert_eq!(custom.rgba(), (0.1, 0.2, 0.3, 0.4));
    }

    #[test]
    fn test_ripple_keyframe_end_time() {
        let kf = RippleKeyframe::new(2.0, NormalizedPoint::CENTER);
        assert!((kf.end_time() - 2.4).abs() < 1e-9); // default duration 0.4
    }

    #[test]
    fn test_ripple_keyframe_is_active() {
        let kf = RippleKeyframe::new(1.0, NormalizedPoint::CENTER);
        assert!(!kf.is_active(0.5));
        assert!(kf.is_active(1.0));
        assert!(kf.is_active(1.2));
        assert!(kf.is_active(1.4));
        assert!(!kf.is_active(1.5));
    }

    #[test]
    fn test_ripple_keyframe_progress() {
        let kf = RippleKeyframe::new(1.0, NormalizedPoint::CENTER);
        assert_eq!(kf.progress(0.5), 0.0); // before start
        assert!((kf.progress(1.2) - 0.5).abs() < 1e-9); // halfway through 0.4s duration
        assert_eq!(kf.progress(2.0), 0.0); // after end
    }

    // CursorStyle tests

    #[test]
    fn test_cursor_style_display_names() {
        assert_eq!(CursorStyle::Arrow.display_name(), "Arrow");
        assert_eq!(CursorStyle::IBeam.display_name(), "I-Beam");
        assert_eq!(CursorStyle::ContextMenu.display_name(), "Context Menu");
    }

    // KeystrokeKeyframe tests

    #[test]
    fn test_keystroke_keyframe_end_time() {
        let kf = KeystrokeKeyframe::new(3.0, "Ctrl+S".into());
        assert!((kf.end_time() - 4.5).abs() < 1e-9); // duration 1.5
    }

    #[test]
    fn test_keystroke_keyframe_is_active() {
        let kf = KeystrokeKeyframe::new(1.0, "Ctrl+C".into());
        assert!(!kf.is_active(0.5));
        assert!(kf.is_active(1.0));
        assert!(kf.is_active(2.0));
        assert!(!kf.is_active(2.6));
    }

    #[test]
    fn test_keystroke_keyframe_opacity_fade_in() {
        let kf = KeystrokeKeyframe::new(0.0, "A".into());
        // At time 0.0 → start of fade-in → opacity ~0
        assert!(kf.opacity(0.0) < 0.01);
        // Halfway through fade-in (0.15s) → ~0.5
        let mid_fade = kf.opacity(0.075);
        assert!((mid_fade - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_keystroke_keyframe_opacity_full() {
        let kf = KeystrokeKeyframe::new(0.0, "B".into());
        // Well past fade-in, before fade-out
        assert_eq!(kf.opacity(0.5), 1.0);
    }

    #[test]
    fn test_keystroke_keyframe_opacity_fade_out() {
        let kf = KeystrokeKeyframe::new(0.0, "C".into());
        // 0.15s before end (end = 1.5, fade_out = 0.3)
        // remaining = 0.15, opacity = 0.15/0.3 = 0.5
        let t = 1.5 - 0.15;
        assert!((kf.opacity(t) - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_keystroke_keyframe_opacity_inactive() {
        let kf = KeystrokeKeyframe::new(5.0, "D".into());
        assert_eq!(kf.opacity(0.0), 0.0);
        assert_eq!(kf.opacity(10.0), 0.0);
    }

    #[test]
    fn test_keystroke_keyframe_progress() {
        let kf = KeystrokeKeyframe::new(0.0, "E".into());
        assert!((kf.progress(0.75) - 0.5).abs() < 0.01); // halfway through 1.5s
    }

    // Serde roundtrip

    #[test]
    fn test_transform_keyframe_serde() {
        let kf = TransformKeyframe::identity(3.0);
        let json = serde_json::to_string(&kf).unwrap();
        let restored: TransformKeyframe = serde_json::from_str(&json).unwrap();
        assert_eq!(kf, restored);
    }

    #[test]
    fn test_ripple_color_serde() {
        let colors = vec![
            RippleColor::LeftClick,
            RippleColor::RightClick,
            RippleColor::Custom { r: 0.1, g: 0.2, b: 0.3, a: 0.4 },
        ];
        for c in &colors {
            let json = serde_json::to_string(c).unwrap();
            let restored: RippleColor = serde_json::from_str(&json).unwrap();
            assert_eq!(c, &restored);
        }
    }
}
