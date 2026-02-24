use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::keyframe::*;

/// Track type identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TrackType {
    Transform,
    Ripple,
    Cursor,
    Keystroke,
}

// MARK: - Transform Track

/// Transform (zoom/pan) track
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TransformTrack {
    pub id: Uuid,
    pub name: String,
    #[serde(rename = "isEnabled")]
    pub is_enabled: bool,
    pub keyframes: Vec<TransformKeyframe>,
}

impl TransformTrack {
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4(),
            name: "Transform".into(),
            is_enabled: true,
            keyframes: Vec::new(),
        }
    }

    pub fn track_type(&self) -> TrackType {
        TrackType::Transform
    }

    pub fn keyframe_count(&self) -> usize {
        self.keyframes.len()
    }

    /// Add keyframe (keep sorted by time)
    pub fn add_keyframe(&mut self, keyframe: TransformKeyframe) {
        self.keyframes.push(keyframe);
        self.keyframes.sort_by(|a, b| a.time.partial_cmp(&b.time).unwrap());
    }

    /// Remove keyframe by id
    pub fn remove_keyframe(&mut self, id: Uuid) {
        self.keyframes.retain(|k| k.id != id);
    }

    /// Update keyframe (re-sort after)
    pub fn update_keyframe(&mut self, keyframe: TransformKeyframe) {
        if let Some(idx) = self.keyframes.iter().position(|k| k.id == keyframe.id) {
            self.keyframes[idx] = keyframe;
            self.keyframes.sort_by(|a, b| a.time.partial_cmp(&b.time).unwrap());
        }
    }

    /// Find keyframe at a specific time (within tolerance)
    pub fn keyframe_at(&self, time: f64, tolerance: f64) -> Option<&TransformKeyframe> {
        self.keyframes.iter().find(|k| (k.time - time).abs() <= tolerance)
    }

    /// Keyframes within a time range
    pub fn keyframes_in_range(&self, start: f64, end: f64) -> Vec<&TransformKeyframe> {
        self.keyframes.iter().filter(|k| k.time >= start && k.time <= end).collect()
    }
}

impl Default for TransformTrack {
    fn default() -> Self {
        Self::new()
    }
}

// MARK: - Ripple Track

/// Ripple effect track
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RippleTrack {
    pub id: Uuid,
    pub name: String,
    #[serde(rename = "isEnabled")]
    pub is_enabled: bool,
    pub keyframes: Vec<RippleKeyframe>,
}

impl RippleTrack {
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4(),
            name: "Click Ripple".into(),
            is_enabled: true,
            keyframes: Vec::new(),
        }
    }

    pub fn track_type(&self) -> TrackType {
        TrackType::Ripple
    }

    pub fn keyframe_count(&self) -> usize {
        self.keyframes.len()
    }

    pub fn add_keyframe(&mut self, keyframe: RippleKeyframe) {
        self.keyframes.push(keyframe);
        self.keyframes.sort_by(|a, b| a.time.partial_cmp(&b.time).unwrap());
    }

    pub fn remove_keyframe(&mut self, id: Uuid) {
        self.keyframes.retain(|k| k.id != id);
    }

    /// Ripples active at the given time
    pub fn active_ripples(&self, time: f64) -> Vec<&RippleKeyframe> {
        self.keyframes.iter().filter(|k| k.is_active(time)).collect()
    }
}

impl Default for RippleTrack {
    fn default() -> Self {
        Self::new()
    }
}

// MARK: - Cursor Track

/// Cursor track
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CursorTrack {
    pub id: Uuid,
    pub name: String,
    #[serde(rename = "isEnabled")]
    pub is_enabled: bool,
    #[serde(rename = "defaultStyle")]
    pub default_style: CursorStyle,
    #[serde(rename = "defaultScale")]
    pub default_scale: f64,
    #[serde(rename = "defaultVisible")]
    pub default_visible: bool,
    #[serde(rename = "styleKeyframes")]
    pub style_keyframes: Option<Vec<CursorStyleKeyframe>>,
}

impl CursorTrack {
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4(),
            name: "Cursor".into(),
            is_enabled: true,
            default_style: CursorStyle::Arrow,
            default_scale: 2.5,
            default_visible: true,
            style_keyframes: None,
        }
    }

    pub fn track_type(&self) -> TrackType {
        TrackType::Cursor
    }

    pub fn keyframe_count(&self) -> usize {
        self.style_keyframes.as_ref().map_or(0, |k| k.len())
    }
}

impl Default for CursorTrack {
    fn default() -> Self {
        Self::new()
    }
}

// MARK: - Keystroke Track

/// Keystroke overlay track
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KeystrokeTrack {
    pub id: Uuid,
    pub name: String,
    #[serde(rename = "isEnabled")]
    pub is_enabled: bool,
    pub keyframes: Vec<KeystrokeKeyframe>,
}

impl KeystrokeTrack {
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4(),
            name: "Keystroke".into(),
            is_enabled: true,
            keyframes: Vec::new(),
        }
    }

    pub fn track_type(&self) -> TrackType {
        TrackType::Keystroke
    }

    pub fn keyframe_count(&self) -> usize {
        self.keyframes.len()
    }

    pub fn add_keyframe(&mut self, keyframe: KeystrokeKeyframe) {
        self.keyframes.push(keyframe);
        self.keyframes.sort_by(|a, b| a.time.partial_cmp(&b.time).unwrap());
    }

    pub fn remove_keyframe(&mut self, id: Uuid) {
        self.keyframes.retain(|k| k.id != id);
    }

    /// Active keystroke overlays at the given time
    pub fn active_keystrokes(&self, time: f64) -> Vec<&KeystrokeKeyframe> {
        self.keyframes.iter().filter(|k| k.is_active(time)).collect()
    }
}

impl Default for KeystrokeTrack {
    fn default() -> Self {
        Self::new()
    }
}

// MARK: - AnyTrack (Type-Erased Wrapper)

/// Type-erased track wrapper for serialization
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum AnyTrack {
    #[serde(rename = "transform")]
    Transform(TransformTrack),
    #[serde(rename = "ripple")]
    Ripple(RippleTrack),
    #[serde(rename = "cursor")]
    Cursor(CursorTrack),
    #[serde(rename = "keystroke")]
    Keystroke(KeystrokeTrack),
}

impl AnyTrack {
    pub fn id(&self) -> Uuid {
        match self {
            Self::Transform(t) => t.id,
            Self::Ripple(t) => t.id,
            Self::Cursor(t) => t.id,
            Self::Keystroke(t) => t.id,
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Self::Transform(t) => &t.name,
            Self::Ripple(t) => &t.name,
            Self::Cursor(t) => &t.name,
            Self::Keystroke(t) => &t.name,
        }
    }

    pub fn is_enabled(&self) -> bool {
        match self {
            Self::Transform(t) => t.is_enabled,
            Self::Ripple(t) => t.is_enabled,
            Self::Cursor(t) => t.is_enabled,
            Self::Keystroke(t) => t.is_enabled,
        }
    }

    pub fn track_type(&self) -> TrackType {
        match self {
            Self::Transform(_) => TrackType::Transform,
            Self::Ripple(_) => TrackType::Ripple,
            Self::Cursor(_) => TrackType::Cursor,
            Self::Keystroke(_) => TrackType::Keystroke,
        }
    }

    pub fn keyframe_count(&self) -> usize {
        match self {
            Self::Transform(t) => t.keyframe_count(),
            Self::Ripple(t) => t.keyframe_count(),
            Self::Cursor(t) => t.keyframe_count(),
            Self::Keystroke(t) => t.keyframe_count(),
        }
    }
}
