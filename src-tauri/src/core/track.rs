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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::coordinates::NormalizedPoint;
    use crate::core::easing::EasingCurve;

    #[test]
    fn test_transform_track_add_keyframe_sorted() {
        let mut track = TransformTrack::new();
        let kf1 = TransformKeyframe::new(2.0, 1.5, NormalizedPoint::CENTER, EasingCurve::Linear);
        let kf2 = TransformKeyframe::new(1.0, 2.0, NormalizedPoint::CENTER, EasingCurve::Linear);
        track.add_keyframe(kf1);
        track.add_keyframe(kf2);
        assert_eq!(track.keyframe_count(), 2);
        assert!(track.keyframes[0].time < track.keyframes[1].time);
    }

    #[test]
    fn test_transform_track_remove_keyframe() {
        let mut track = TransformTrack::new();
        let kf = TransformKeyframe::new(1.0, 1.0, NormalizedPoint::CENTER, EasingCurve::Linear);
        let id = kf.id;
        track.add_keyframe(kf);
        assert_eq!(track.keyframe_count(), 1);
        track.remove_keyframe(id);
        assert_eq!(track.keyframe_count(), 0);
    }

    #[test]
    fn test_transform_track_update_keyframe_resorts() {
        let mut track = TransformTrack::new();
        let kf1 = TransformKeyframe::new(1.0, 1.0, NormalizedPoint::CENTER, EasingCurve::Linear);
        let kf2 = TransformKeyframe::new(3.0, 1.0, NormalizedPoint::CENTER, EasingCurve::Linear);
        let id2 = kf2.id;
        track.add_keyframe(kf1);
        track.add_keyframe(kf2);
        // Move kf2 to time 0.5 — it should become first
        let mut updated = track.keyframes[1].clone();
        updated.time = 0.5;
        track.update_keyframe(updated);
        assert_eq!(track.keyframes[0].id, id2);
    }

    #[test]
    fn test_transform_track_keyframe_at() {
        let mut track = TransformTrack::new();
        track.add_keyframe(TransformKeyframe::new(
            5.0, 1.5, NormalizedPoint::CENTER, EasingCurve::Linear,
        ));
        assert!(track.keyframe_at(5.0, 0.01).is_some());
        assert!(track.keyframe_at(5.005, 0.01).is_some());
        assert!(track.keyframe_at(6.0, 0.01).is_none());
    }

    #[test]
    fn test_transform_track_keyframes_in_range() {
        let mut track = TransformTrack::new();
        for t in [1.0, 3.0, 5.0, 7.0] {
            track.add_keyframe(TransformKeyframe::new(
                t, 1.0, NormalizedPoint::CENTER, EasingCurve::Linear,
            ));
        }
        let range = track.keyframes_in_range(2.0, 6.0);
        assert_eq!(range.len(), 2); // 3.0 and 5.0
    }

    #[test]
    fn test_ripple_track_active_ripples() {
        let mut track = RippleTrack::new();
        let mut kf = RippleKeyframe::new(1.0, NormalizedPoint::CENTER);
        kf.duration = 0.5;
        track.add_keyframe(kf);
        assert_eq!(track.active_ripples(1.2).len(), 1);
        assert_eq!(track.active_ripples(2.0).len(), 0);
    }

    #[test]
    fn test_cursor_track_defaults() {
        let track = CursorTrack::new();
        assert_eq!(track.default_style, CursorStyle::Arrow);
        assert_eq!(track.default_scale, 2.5);
        assert!(track.default_visible);
        assert_eq!(track.keyframe_count(), 0);
    }

    #[test]
    fn test_keystroke_track_active_keystrokes() {
        let mut track = KeystrokeTrack::new();
        let kf = KeystrokeKeyframe::new(2.0, "Ctrl+C".into());
        track.add_keyframe(kf);
        assert_eq!(track.active_keystrokes(2.5).len(), 1);
        assert_eq!(track.active_keystrokes(5.0).len(), 0);
    }

    #[test]
    fn test_any_track_delegates() {
        let track = TransformTrack::new();
        let id = track.id;
        let any = AnyTrack::Transform(track);
        assert_eq!(any.id(), id);
        assert_eq!(any.name(), "Transform");
        assert!(any.is_enabled());
        assert_eq!(any.track_type(), TrackType::Transform);
        assert_eq!(any.keyframe_count(), 0);
    }

    #[test]
    fn test_any_track_serde_roundtrip() {
        let mut track = RippleTrack::new();
        track.add_keyframe(RippleKeyframe::new(1.0, NormalizedPoint::new(0.3, 0.7)));
        let any = AnyTrack::Ripple(track);
        let json = serde_json::to_string(&any).unwrap();
        let restored: AnyTrack = serde_json::from_str(&json).unwrap();
        assert_eq!(any, restored);
    }
}
