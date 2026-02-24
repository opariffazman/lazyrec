use serde::{Deserialize, Serialize};

use super::coordinates::NormalizedPoint;
use super::keyframe::*;
use super::timeline::Timeline;
use super::track::*;

/// Evaluated state of all tracks at a single point in time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluatedFrameState {
    pub time: f64,
    pub transform: TransformState,
    pub ripples: Vec<ActiveRipple>,
    pub cursor: CursorState,
    pub keystrokes: Vec<ActiveKeystroke>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformState {
    pub zoom: f64,
    pub center: NormalizedPoint,
    /// Velocity (from easing derivative), used for motion blur
    pub velocity: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveRipple {
    pub position: NormalizedPoint,
    pub progress: f64,
    pub intensity: f64,
    pub color: (f64, f64, f64, f64),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CursorState {
    pub position: NormalizedPoint,
    pub style: CursorStyle,
    pub scale: f64,
    pub visible: bool,
    pub velocity: f64,
    pub movement_direction: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveKeystroke {
    pub display_text: String,
    pub position: NormalizedPoint,
    pub opacity: f64,
}

/// Mouse position for cursor interpolation
#[derive(Debug, Clone)]
pub struct MousePosition {
    pub time: f64,
    pub position: NormalizedPoint,
}

/// Frame evaluator: evaluates timeline state at any point in time
pub struct FrameEvaluator {
    pub window_mode: bool,
}

impl FrameEvaluator {
    pub fn new(window_mode: bool) -> Self {
        Self { window_mode }
    }

    /// Evaluate all tracks at the given time
    pub fn evaluate(
        &self,
        timeline: &Timeline,
        time: f64,
        mouse_positions: &[MousePosition],
    ) -> EvaluatedFrameState {
        let transform = self.evaluate_transform(timeline.transform_track(), time);
        let ripples = self.evaluate_ripples(timeline.ripple_track(), time);
        let cursor = self.evaluate_cursor(
            timeline.cursor_track(),
            time,
            mouse_positions,
        );
        let keystrokes = self.evaluate_keystrokes(timeline.keystroke_track(), time);

        EvaluatedFrameState {
            time,
            transform,
            ripples,
            cursor,
            keystrokes,
        }
    }

    /// Evaluate transform track: binary search + easing interpolation
    fn evaluate_transform(
        &self,
        track: Option<&TransformTrack>,
        time: f64,
    ) -> TransformState {
        let track = match track {
            Some(t) if t.is_enabled && !t.keyframes.is_empty() => t,
            _ => {
                return TransformState {
                    zoom: 1.0,
                    center: NormalizedPoint::CENTER,
                    velocity: 0.0,
                };
            }
        };

        let keyframes = &track.keyframes;

        // Before first keyframe
        if time <= keyframes[0].time {
            return TransformState {
                zoom: keyframes[0].zoom,
                center: keyframes[0].center,
                velocity: 0.0,
            };
        }

        // After last keyframe
        if time >= keyframes[keyframes.len() - 1].time {
            let last = &keyframes[keyframes.len() - 1];
            return TransformState {
                zoom: last.zoom,
                center: last.center,
                velocity: 0.0,
            };
        }

        // Binary search for bounding keyframes
        let (from, to) = find_bounding_keyframes(keyframes, time);
        let from_kf = &keyframes[from];
        let to_kf = &keyframes[to];

        let segment_duration = to_kf.time - from_kf.time;
        if segment_duration <= 0.001 {
            return TransformState {
                zoom: to_kf.zoom,
                center: to_kf.center,
                velocity: 0.0,
            };
        }

        let t = (time - from_kf.time) / segment_duration;
        let eased_t = from_kf.easing.apply(t, segment_duration);

        let from_val = from_kf.value();
        let to_val = to_kf.value();

        let interpolated = if self.window_mode {
            from_val.interpolated_for_window_mode(&to_val, eased_t)
        } else {
            from_val.interpolated(&to_val, eased_t)
        };

        // Clamp center to valid range based on zoom (prevents crop exceeding image)
        let center = if !self.window_mode && interpolated.zoom > 1.0 {
            clamp_center(interpolated.center, interpolated.zoom)
        } else {
            interpolated.center
        };

        // Compute velocity from easing derivative
        let velocity = from_kf.easing.derivative(t, segment_duration);

        TransformState {
            zoom: interpolated.zoom,
            center,
            velocity,
        }
    }

    /// Evaluate ripple track: find all active ripples
    fn evaluate_ripples(
        &self,
        track: Option<&RippleTrack>,
        time: f64,
    ) -> Vec<ActiveRipple> {
        let track = match track {
            Some(t) if t.is_enabled => t,
            _ => return Vec::new(),
        };

        track
            .keyframes
            .iter()
            .filter(|k| k.is_active(time))
            .map(|k| {
                let raw_progress = k.progress(time);
                let eased_progress = k.easing.apply(raw_progress, k.duration);
                ActiveRipple {
                    position: k.position,
                    progress: eased_progress,
                    intensity: k.intensity,
                    color: k.color.rgba(),
                }
            })
            .collect()
    }

    /// Evaluate cursor track: interpolate position + discrete style
    fn evaluate_cursor(
        &self,
        track: Option<&CursorTrack>,
        time: f64,
        mouse_positions: &[MousePosition],
    ) -> CursorState {
        let default = CursorState {
            position: interpolate_mouse_position(mouse_positions, time),
            style: CursorStyle::Arrow,
            scale: 2.5,
            visible: true,
            velocity: 0.0,
            movement_direction: 0.0,
        };

        let track = match track {
            Some(t) if t.is_enabled => t,
            _ => return default,
        };

        let keyframes = match &track.style_keyframes {
            Some(kfs) if !kfs.is_empty() => kfs,
            _ => return default,
        };

        // Find last keyframe at or before time (discrete interpolation for style/visibility)
        let active_kf = keyframes
            .iter()
            .rev()
            .find(|k| k.time <= time);

        let (style, visible, scale) = match active_kf {
            Some(kf) => (kf.style, kf.visible, kf.scale),
            None => (track.default_style, track.default_visible, track.default_scale),
        };

        // Position: use keyframe position if available, otherwise raw mouse
        let position = active_kf
            .and_then(|kf| kf.position)
            .unwrap_or_else(|| interpolate_mouse_position(mouse_positions, time));

        // Velocity and direction from keyframe or computed from mouse
        let (velocity, direction) = active_kf
            .map(|kf| (kf.velocity.unwrap_or(0.0), kf.movement_direction.unwrap_or(0.0)))
            .unwrap_or((0.0, 0.0));

        CursorState {
            position,
            style,
            scale,
            visible,
            velocity,
            movement_direction: direction,
        }
    }

    /// Evaluate keystroke track: find all active keystroke overlays
    fn evaluate_keystrokes(
        &self,
        track: Option<&KeystrokeTrack>,
        time: f64,
    ) -> Vec<ActiveKeystroke> {
        let track = match track {
            Some(t) if t.is_enabled => t,
            _ => return Vec::new(),
        };

        track
            .keyframes
            .iter()
            .filter(|k| k.is_active(time))
            .map(|k| ActiveKeystroke {
                display_text: k.display_text.clone(),
                position: k.position,
                opacity: k.opacity(time),
            })
            .collect()
    }
}

/// Clamp center to valid range based on zoom level.
/// Prevents the crop rectangle from exceeding the normalized image bounds.
fn clamp_center(center: NormalizedPoint, zoom: f64) -> NormalizedPoint {
    let half_crop = 0.5 / zoom;
    NormalizedPoint {
        x: center.x.clamp(half_crop, 1.0 - half_crop),
        y: center.y.clamp(half_crop, 1.0 - half_crop),
    }
}

/// Binary search for bounding keyframe indices around query time.
/// Returns (from_index, to_index) where keyframes[from].time <= time < keyframes[to].time
fn find_bounding_keyframes(keyframes: &[TransformKeyframe], time: f64) -> (usize, usize) {
    debug_assert!(keyframes.len() >= 2);

    let mut lo = 0;
    let mut hi = keyframes.len() - 1;

    while lo < hi - 1 {
        let mid = (lo + hi) / 2;
        if keyframes[mid].time <= time {
            lo = mid;
        } else {
            hi = mid;
        }
    }

    (lo, hi)
}

/// Interpolate mouse position using Catmull-Rom spline.
/// Falls back to linear interpolation for < 4 data points.
fn interpolate_mouse_position(positions: &[MousePosition], time: f64) -> NormalizedPoint {
    if positions.is_empty() {
        return NormalizedPoint::CENTER;
    }

    if positions.len() == 1 {
        return positions[0].position;
    }

    // Binary search for the segment
    let idx = match positions.binary_search_by(|p| p.time.partial_cmp(&time).unwrap()) {
        Ok(i) => return positions[i].position,
        Err(i) => i,
    };

    if idx == 0 {
        return positions[0].position;
    }
    if idx >= positions.len() {
        return positions[positions.len() - 1].position;
    }

    // Linear interpolation for few points
    if positions.len() < 4 {
        let a = &positions[idx - 1];
        let b = &positions[idx];
        let seg = b.time - a.time;
        if seg <= 0.0 {
            return a.position;
        }
        let t = (time - a.time) / seg;
        return a.position.interpolated(&b.position, t);
    }

    // Catmull-Rom spline with 4 control points
    let i1 = idx - 1;
    let i2 = idx;
    let i0 = if i1 > 0 { i1 - 1 } else { i1 };
    let i3 = (i2 + 1).min(positions.len() - 1);

    let seg = positions[i2].time - positions[i1].time;
    if seg <= 0.0 {
        return positions[i1].position;
    }
    let t = (time - positions[i1].time) / seg;

    let p0 = positions[i0].position;
    let p1 = positions[i1].position;
    let p2 = positions[i2].position;
    let p3 = positions[i3].position;

    let result = catmull_rom(p0, p1, p2, p3, t, 0.2);
    result.clamped()
}

/// Catmull-Rom spline interpolation between p1 and p2.
/// tension: 0.2 (reduced from 0.5 for smoother curves)
fn catmull_rom(
    p0: NormalizedPoint,
    p1: NormalizedPoint,
    p2: NormalizedPoint,
    p3: NormalizedPoint,
    t: f64,
    tension: f64,
) -> NormalizedPoint {
    let t2 = t * t;
    let t3 = t2 * t;

    let x = catmull_rom_1d(p0.x, p1.x, p2.x, p3.x, t, t2, t3, tension);
    let y = catmull_rom_1d(p0.y, p1.y, p2.y, p3.y, t, t2, t3, tension);

    NormalizedPoint::new(x, y)
}

fn catmull_rom_1d(
    p0: f64, p1: f64, p2: f64, p3: f64,
    t: f64, t2: f64, t3: f64,
    tension: f64,
) -> f64 {
    let a0 = -tension * p0 + (2.0 - tension) * p1 + (tension - 2.0) * p2 + tension * p3;
    let a1 = 2.0 * tension * p0 + (tension - 3.0) * p1 + (3.0 - 2.0 * tension) * p2 - tension * p3;
    let a2 = -tension * p0 + tension * p2;
    let a3 = p1;
    a0 * t3 + a1 * t2 + a2 * t + a3
}

/// Interpolate between two angles, handling wrap-around
pub fn interpolate_angle(a1: f64, a2: f64, t: f64) -> f64 {
    let pi = std::f64::consts::PI;
    let mut diff = a2 - a1;
    while diff > pi {
        diff -= 2.0 * pi;
    }
    while diff < -pi {
        diff += 2.0 * pi;
    }
    a1 + diff * t
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::easing::EasingCurve;

    #[test]
    fn test_clamp_center() {
        // At 2x zoom, half crop = 0.25, valid range is [0.25, 0.75]
        let c = clamp_center(NormalizedPoint::new(0.1, 0.9), 2.0);
        assert!((c.x - 0.25).abs() < 1e-10);
        assert!((c.y - 0.75).abs() < 1e-10);
    }

    #[test]
    fn test_find_bounding_keyframes() {
        let kfs = vec![
            TransformKeyframe::identity(0.0),
            TransformKeyframe::identity(1.0),
            TransformKeyframe::identity(2.0),
            TransformKeyframe::identity(3.0),
        ];
        let (lo, hi) = find_bounding_keyframes(&kfs, 1.5);
        assert_eq!(lo, 1);
        assert_eq!(hi, 2);
    }

    #[test]
    fn test_interpolate_mouse_linear() {
        let positions = vec![
            MousePosition { time: 0.0, position: NormalizedPoint::new(0.0, 0.0) },
            MousePosition { time: 1.0, position: NormalizedPoint::new(1.0, 1.0) },
        ];
        let p = interpolate_mouse_position(&positions, 0.5);
        assert!((p.x - 0.5).abs() < 1e-10);
        assert!((p.y - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_interpolate_mouse_catmull_rom() {
        let positions = vec![
            MousePosition { time: 0.0, position: NormalizedPoint::new(0.0, 0.0) },
            MousePosition { time: 1.0, position: NormalizedPoint::new(0.25, 0.25) },
            MousePosition { time: 2.0, position: NormalizedPoint::new(0.75, 0.75) },
            MousePosition { time: 3.0, position: NormalizedPoint::new(1.0, 1.0) },
        ];
        let p = interpolate_mouse_position(&positions, 1.5);
        // Should be between 0.25 and 0.75 (smooth curve)
        assert!(p.x > 0.2 && p.x < 0.8);
        assert!(p.y > 0.2 && p.y < 0.8);
    }

    #[test]
    fn test_evaluate_transform_single_keyframe() {
        let mut track = TransformTrack::new();
        track.add_keyframe(TransformKeyframe::new(
            1.0,
            2.0,
            NormalizedPoint::new(0.3, 0.4),
            EasingCurve::Linear,
        ));

        let evaluator = FrameEvaluator::new(false);
        let state = evaluator.evaluate_transform(Some(&track), 0.5);
        assert!((state.zoom - 2.0).abs() < 1e-10);
    }

    #[test]
    fn test_evaluate_transform_interpolation() {
        let mut track = TransformTrack::new();
        track.add_keyframe(TransformKeyframe::new(
            0.0, 1.0, NormalizedPoint::CENTER, EasingCurve::Linear,
        ));
        track.add_keyframe(TransformKeyframe::new(
            1.0, 3.0, NormalizedPoint::CENTER, EasingCurve::Linear,
        ));

        let evaluator = FrameEvaluator::new(false);
        let state = evaluator.evaluate_transform(Some(&track), 0.5);
        assert!((state.zoom - 2.0).abs() < 1e-10);
    }

    #[test]
    fn test_interpolate_angle_wraparound() {
        let pi = std::f64::consts::PI;
        let result = interpolate_angle(pi * 0.9, -pi * 0.9, 0.5);
        // Should go through pi (the short way around), not through 0
        assert!(result.abs() > pi * 0.8);
    }
}
