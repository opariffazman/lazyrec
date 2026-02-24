use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::track::*;

/// Timeline contains multiple tracks, each holding keyframes
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Timeline {
    pub tracks: Vec<AnyTrack>,
    /// Total timeline duration (seconds)
    pub duration: f64,
    /// Trim start time
    #[serde(default, rename = "trimStart")]
    pub trim_start: f64,
    /// Trim end time (None uses duration)
    #[serde(default, rename = "trimEnd")]
    pub trim_end: Option<f64>,
}

impl Timeline {
    pub fn new(duration: f64) -> Self {
        Self {
            tracks: Vec::new(),
            duration,
            trim_start: 0.0,
            trim_end: None,
        }
    }

    /// Create a timeline initialized with default tracks
    pub fn with_default_tracks(duration: f64) -> Self {
        Self {
            tracks: vec![
                AnyTrack::Transform(TransformTrack::new()),
                AnyTrack::Ripple(RippleTrack::new()),
                AnyTrack::Cursor(CursorTrack::new()),
                AnyTrack::Keystroke(KeystrokeTrack::new()),
            ],
            duration,
            trim_start: 0.0,
            trim_end: None,
        }
    }

    // Trim properties

    pub fn effective_trim_start(&self) -> f64 {
        self.trim_start.max(0.0).min(self.duration)
    }

    pub fn effective_trim_end(&self) -> f64 {
        self.trim_end.unwrap_or(self.duration).min(self.duration)
    }

    pub fn trimmed_duration(&self) -> f64 {
        (self.effective_trim_end() - self.effective_trim_start()).max(0.0)
    }

    pub fn is_trimmed(&self) -> bool {
        self.effective_trim_start() > 0.0 || self.effective_trim_end() < self.duration
    }

    pub fn is_time_in_trim_range(&self, time: f64) -> bool {
        time >= self.effective_trim_start() && time <= self.effective_trim_end()
    }

    // Track access

    pub fn transform_track(&self) -> Option<&TransformTrack> {
        self.tracks.iter().find_map(|t| {
            if let AnyTrack::Transform(track) = t { Some(track) } else { None }
        })
    }

    pub fn transform_track_mut(&mut self) -> Option<&mut TransformTrack> {
        self.tracks.iter_mut().find_map(|t| {
            if let AnyTrack::Transform(track) = t { Some(track) } else { None }
        })
    }

    pub fn ripple_track(&self) -> Option<&RippleTrack> {
        self.tracks.iter().find_map(|t| {
            if let AnyTrack::Ripple(track) = t { Some(track) } else { None }
        })
    }

    pub fn ripple_track_mut(&mut self) -> Option<&mut RippleTrack> {
        self.tracks.iter_mut().find_map(|t| {
            if let AnyTrack::Ripple(track) = t { Some(track) } else { None }
        })
    }

    pub fn cursor_track(&self) -> Option<&CursorTrack> {
        self.tracks.iter().find_map(|t| {
            if let AnyTrack::Cursor(track) = t { Some(track) } else { None }
        })
    }

    pub fn keystroke_track(&self) -> Option<&KeystrokeTrack> {
        self.tracks.iter().find_map(|t| {
            if let AnyTrack::Keystroke(track) = t { Some(track) } else { None }
        })
    }

    pub fn keystroke_track_mut(&mut self) -> Option<&mut KeystrokeTrack> {
        self.tracks.iter_mut().find_map(|t| {
            if let AnyTrack::Keystroke(track) = t { Some(track) } else { None }
        })
    }

    // Track management

    pub fn add_track(&mut self, track: AnyTrack) {
        self.tracks.push(track);
    }

    pub fn remove_track(&mut self, id: Uuid) {
        self.tracks.retain(|t| t.id() != id);
    }

    pub fn track(&self, id: Uuid) -> Option<&AnyTrack> {
        self.tracks.iter().find(|t| t.id() == id)
    }

    pub fn update_track(&mut self, track: AnyTrack) {
        if let Some(idx) = self.tracks.iter().position(|t| t.id() == track.id()) {
            self.tracks[idx] = track;
        }
    }

    // Keyframe query

    pub fn total_keyframe_count(&self) -> usize {
        self.tracks.iter().map(|t| t.keyframe_count()).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.total_keyframe_count() == 0
    }

    pub fn is_valid(&self) -> bool {
        self.duration > 0.0
    }
}

impl Default for Timeline {
    fn default() -> Self {
        Self::new(0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_timeline() {
        let tl = Timeline::new(10.0);
        assert_eq!(tl.duration, 10.0);
        assert!(tl.tracks.is_empty());
        assert_eq!(tl.trim_start, 0.0);
        assert_eq!(tl.trim_end, None);
    }

    #[test]
    fn test_with_default_tracks() {
        let tl = Timeline::with_default_tracks(30.0);
        assert_eq!(tl.tracks.len(), 4);
        assert_eq!(tl.tracks[0].track_type(), TrackType::Transform);
        assert_eq!(tl.tracks[1].track_type(), TrackType::Ripple);
        assert_eq!(tl.tracks[2].track_type(), TrackType::Cursor);
        assert_eq!(tl.tracks[3].track_type(), TrackType::Keystroke);
    }

    #[test]
    fn test_effective_trim_start_clamped() {
        let mut tl = Timeline::new(10.0);
        tl.trim_start = -5.0;
        assert_eq!(tl.effective_trim_start(), 0.0);
        tl.trim_start = 15.0;
        assert_eq!(tl.effective_trim_start(), 10.0);
    }

    #[test]
    fn test_effective_trim_end_default_and_clamped() {
        let mut tl = Timeline::new(10.0);
        assert_eq!(tl.effective_trim_end(), 10.0);
        tl.trim_end = Some(5.0);
        assert_eq!(tl.effective_trim_end(), 5.0);
        tl.trim_end = Some(20.0);
        assert_eq!(tl.effective_trim_end(), 10.0);
    }

    #[test]
    fn test_trimmed_duration() {
        let mut tl = Timeline::new(10.0);
        assert_eq!(tl.trimmed_duration(), 10.0);
        tl.trim_start = 2.0;
        tl.trim_end = Some(8.0);
        assert_eq!(tl.trimmed_duration(), 6.0);
    }

    #[test]
    fn test_is_trimmed() {
        let mut tl = Timeline::new(10.0);
        assert!(!tl.is_trimmed());
        tl.trim_start = 1.0;
        assert!(tl.is_trimmed());
    }

    #[test]
    fn test_is_time_in_trim_range() {
        let mut tl = Timeline::new(10.0);
        tl.trim_start = 2.0;
        tl.trim_end = Some(8.0);
        assert!(!tl.is_time_in_trim_range(1.0));
        assert!(tl.is_time_in_trim_range(5.0));
        assert!(!tl.is_time_in_trim_range(9.0));
    }

    #[test]
    fn test_track_access_typed() {
        let tl = Timeline::with_default_tracks(10.0);
        assert!(tl.transform_track().is_some());
        assert!(tl.ripple_track().is_some());
        assert!(tl.cursor_track().is_some());
        assert!(tl.keystroke_track().is_some());
    }

    #[test]
    fn test_add_and_remove_track() {
        let mut tl = Timeline::new(10.0);
        let track = TransformTrack::new();
        let id = track.id;
        tl.add_track(AnyTrack::Transform(track));
        assert_eq!(tl.tracks.len(), 1);
        tl.remove_track(id);
        assert!(tl.tracks.is_empty());
    }

    #[test]
    fn test_track_by_id() {
        let mut tl = Timeline::new(10.0);
        let track = RippleTrack::new();
        let id = track.id;
        tl.add_track(AnyTrack::Ripple(track));
        assert!(tl.track(id).is_some());
        assert!(tl.track(Uuid::new_v4()).is_none());
    }

    #[test]
    fn test_total_keyframe_count_empty() {
        let tl = Timeline::with_default_tracks(10.0);
        assert_eq!(tl.total_keyframe_count(), 0);
        assert!(tl.is_empty());
    }

    #[test]
    fn test_is_valid() {
        assert!(Timeline::new(10.0).is_valid());
        assert!(!Timeline::new(0.0).is_valid());
        assert!(!Timeline::new(-1.0).is_valid());
    }

    #[test]
    fn test_default_timeline() {
        let tl = Timeline::default();
        assert_eq!(tl.duration, 0.0);
        assert!(tl.tracks.is_empty());
    }

    #[test]
    fn test_serde_roundtrip() {
        let tl = Timeline::with_default_tracks(15.0);
        let json = serde_json::to_string(&tl).unwrap();
        let restored: Timeline = serde_json::from_str(&json).unwrap();
        assert_eq!(tl, restored);
    }
}
