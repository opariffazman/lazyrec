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
