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
