//! Keyframe generators: auto-generate keyframes from mouse/keyboard data.

use uuid::Uuid;

use super::coordinates::NormalizedPoint;
use super::easing::EasingCurve;
use super::keyframe::*;
use super::track::*;

// ============================================================================
// Mouse Data Types (input to generators)
// ============================================================================

/// Click type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClickType {
    LeftDown,
    LeftUp,
    RightDown,
    RightUp,
    DoubleClick,
}

/// Click event from recording
#[derive(Debug, Clone)]
pub struct ClickEvent {
    pub time: f64,
    pub position: NormalizedPoint,
    pub click_type: ClickType,
    pub duration: f64,
}

/// Keyboard event from recording
#[derive(Debug, Clone)]
pub struct KeyboardEvent {
    pub time: f64,
    pub event_type: KeyEventType,
    pub key_code: u16,
    pub character: Option<String>,
    pub modifiers: Modifiers,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyEventType {
    KeyDown,
    KeyUp,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Modifiers {
    pub command: bool,
    pub shift: bool,
    pub alt: bool,
    pub control: bool,
}

impl Modifiers {
    pub fn has_modifiers(&self) -> bool {
        self.command || self.shift || self.alt || self.control
    }
}

/// Drag event from recording
#[derive(Debug, Clone)]
pub struct DragEvent {
    pub start_time: f64,
    pub end_time: f64,
    pub start_position: NormalizedPoint,
    pub end_position: NormalizedPoint,
}

/// Complete mouse data source for generators
pub struct MouseData {
    pub positions: Vec<(f64, NormalizedPoint)>,
    pub clicks: Vec<ClickEvent>,
    pub keyboard_events: Vec<KeyboardEvent>,
    pub drags: Vec<DragEvent>,
    pub duration: f64,
}

// ============================================================================
// Activity Collector (SmartZoom input)
// ============================================================================

/// Activity type detected from mouse data
#[derive(Debug, Clone, PartialEq)]
pub enum ActivityType {
    Click,
    Typing,
    DragStart,
    DragEnd,
}

/// Single activity event
#[derive(Debug, Clone)]
pub struct ActivityEvent {
    pub time: f64,
    pub position: NormalizedPoint,
    pub activity_type: ActivityType,
}

/// Collect activities from raw mouse data
pub fn collect_activities(data: &MouseData) -> Vec<ActivityEvent> {
    let mut activities: Vec<ActivityEvent> = Vec::new();

    // Clicks (left down only)
    for click in &data.clicks {
        if click.click_type == ClickType::LeftDown {
            activities.push(ActivityEvent {
                time: click.time,
                position: click.position,
                activity_type: ActivityType::Click,
            });
        }
    }

    // Drags
    for drag in &data.drags {
        activities.push(ActivityEvent {
            time: drag.start_time,
            position: drag.start_position,
            activity_type: ActivityType::DragStart,
        });
        activities.push(ActivityEvent {
            time: drag.end_time,
            position: drag.end_position,
            activity_type: ActivityType::DragEnd,
        });
    }

    // Typing sessions
    let sessions = detect_typing_sessions(&data.keyboard_events);
    for session in sessions {
        // Find cursor position at session start
        let position = find_position_at_time(&data.positions, session.start);
        activities.push(ActivityEvent {
            time: session.start,
            position,
            activity_type: ActivityType::Typing,
        });
        if session.end - session.start > 0.5 {
            activities.push(ActivityEvent {
                time: session.end,
                position,
                activity_type: ActivityType::Typing,
            });
        }
    }

    activities.sort_by(|a, b| a.time.partial_cmp(&b.time).unwrap());
    activities
}

struct TypingSession {
    start: f64,
    end: f64,
}

const TYPING_SESSION_TIMEOUT: f64 = 1.5;

fn detect_typing_sessions(events: &[KeyboardEvent]) -> Vec<TypingSession> {
    let key_downs: Vec<&KeyboardEvent> = events
        .iter()
        .filter(|e| e.event_type == KeyEventType::KeyDown && !e.modifiers.has_modifiers())
        .collect();

    if key_downs.is_empty() {
        return Vec::new();
    }

    let mut sessions = Vec::new();
    let mut session_start = key_downs[0].time;
    let mut last_time = session_start;

    for event in &key_downs[1..] {
        if event.time - last_time > TYPING_SESSION_TIMEOUT {
            sessions.push(TypingSession { start: session_start, end: last_time });
            session_start = event.time;
        }
        last_time = event.time;
    }
    sessions.push(TypingSession { start: session_start, end: last_time });

    sessions
}

fn find_position_at_time(positions: &[(f64, NormalizedPoint)], time: f64) -> NormalizedPoint {
    if positions.is_empty() {
        return NormalizedPoint::CENTER;
    }

    // Find last position at or before time
    match positions.binary_search_by(|p| p.0.partial_cmp(&time).unwrap()) {
        Ok(i) => positions[i].1,
        Err(0) => positions[0].1,
        Err(i) => positions[i - 1].1,
    }
}

// ============================================================================
// Session Clusterer
// ============================================================================

/// Work session: a group of related activities
#[derive(Debug, Clone)]
pub struct WorkSession {
    pub start_time: f64,
    pub end_time: f64,
    pub activities: Vec<ActivityEvent>,
    /// Bounding box: (min_x, min_y, width, height) in normalized coords
    pub work_area: (f64, f64, f64, f64),
    pub center: NormalizedPoint,
    pub zoom: f64,
}

impl WorkSession {
    fn from_activity(activity: &ActivityEvent, padding: f64) -> Self {
        let (min_x, min_y, w, h) = padded_bbox(activity.position, activity.position, padding);
        Self {
            start_time: activity.time,
            end_time: activity.time,
            activities: vec![activity.clone()],
            work_area: (min_x, min_y, w, h),
            center: activity.position,
            zoom: 1.0,
        }
    }

    fn add_activity(&mut self, activity: &ActivityEvent, padding: f64) {
        self.activities.push(activity.clone());
        self.end_time = activity.time;
        self.update_work_area(activity.position, padding);
    }

    fn update_work_area(&mut self, position: NormalizedPoint, padding: f64) {
        let (cur_x, cur_y, cur_w, cur_h) = self.work_area;
        let cur_max_x = cur_x + cur_w;
        let cur_max_y = cur_y + cur_h;

        let min_x = (cur_x.min(position.x - padding)).max(0.0);
        let min_y = (cur_y.min(position.y - padding)).max(0.0);
        let max_x = (cur_max_x.max(position.x + padding)).min(1.0);
        let max_y = (cur_max_y.max(position.y + padding)).min(1.0);

        self.work_area = (min_x, min_y, max_x - min_x, max_y - min_y);
        self.center = NormalizedPoint::new(
            (min_x + max_x) / 2.0,
            (min_y + max_y) / 2.0,
        );
    }
}

fn padded_bbox(a: NormalizedPoint, b: NormalizedPoint, padding: f64) -> (f64, f64, f64, f64) {
    let min_x = (a.x.min(b.x) - padding).max(0.0);
    let min_y = (a.y.min(b.y) - padding).max(0.0);
    let max_x = (a.x.max(b.x) + padding).min(1.0);
    let max_y = (a.y.max(b.y) + padding).min(1.0);
    (min_x, min_y, max_x - min_x, max_y - min_y)
}

/// SmartZoom settings
#[derive(Debug, Clone)]
pub struct SmartZoomSettings {
    pub min_zoom: f64,
    pub max_zoom: f64,
    pub default_zoom: f64,
    pub target_area_coverage: f64,
    pub focusing_duration: f64,
    pub idle_timeout: f64,
    pub transition_duration: f64,
    pub session_merge_interval: f64,
    pub session_merge_distance: f64,
    pub work_area_padding: f64,
    pub zoom_in_easing: EasingCurve,
    pub zoom_out_easing: EasingCurve,
    pub move_easing: EasingCurve,
}

impl Default for SmartZoomSettings {
    fn default() -> Self {
        Self {
            min_zoom: 2.0,
            max_zoom: 8.0,
            default_zoom: 4.0,
            target_area_coverage: 0.4,
            focusing_duration: 0.5,
            idle_timeout: 1.0,
            transition_duration: 0.6,
            session_merge_interval: 2.0,
            session_merge_distance: 0.15,
            work_area_padding: 0.02,
            zoom_in_easing: EasingCurve::spring_default(),
            zoom_out_easing: EasingCurve::EaseInOut,
            move_easing: EasingCurve::spring_smooth(),
        }
    }
}

/// Cluster activities into work sessions
pub fn cluster_activities(
    activities: &[ActivityEvent],
    settings: &SmartZoomSettings,
) -> Vec<WorkSession> {
    if activities.is_empty() {
        return Vec::new();
    }

    let mut sessions: Vec<WorkSession> = Vec::new();
    let mut current = WorkSession::from_activity(&activities[0], settings.work_area_padding);

    for activity in &activities[1..] {
        let time_delta = activity.time - current.end_time;
        let distance = activity.position.distance(&current.center);

        let is_continuous_typing = activity.activity_type == ActivityType::Typing
            && current.activities.last().map_or(false, |a| a.activity_type == ActivityType::Typing)
            && distance < settings.session_merge_distance;

        let should_merge = is_continuous_typing
            || (time_delta < settings.session_merge_interval
                && distance < settings.session_merge_distance);

        if should_merge {
            current.add_activity(activity, settings.work_area_padding);
        } else {
            sessions.push(current);
            current = WorkSession::from_activity(activity, settings.work_area_padding);
        }
    }
    sessions.push(current);

    sessions
}

// ============================================================================
// Zoom Level Calculator
// ============================================================================

/// Calculate zoom level for a session based on its work area
pub fn calculate_session_zoom(session: &mut WorkSession, settings: &SmartZoomSettings) {
    let (_, _, w, h) = session.work_area;
    let area_size = w.max(h);

    if area_size <= 0.01 {
        session.zoom = settings.default_zoom;
        return;
    }

    // Cap the area_size so spread-out clicks still get meaningful zoom
    // Even if clicks span the whole screen, zoom at least to target_area_coverage
    let effective_area = area_size.min(settings.target_area_coverage);

    let zoom = settings.target_area_coverage / effective_area;
    session.zoom = zoom.clamp(settings.min_zoom, settings.max_zoom);
}

// ============================================================================
// SmartZoom Generator (orchestrator)
// ============================================================================

/// Generate smart zoom transform keyframes from mouse data
pub fn generate_smart_zoom(
    data: &MouseData,
    settings: &SmartZoomSettings,
) -> TransformTrack {
    let activities = collect_activities(data);
    let mut sessions = cluster_activities(&activities, settings);

    // Calculate zoom for each session
    for session in &mut sessions {
        calculate_session_zoom(session, settings);
    }

    // Generate keyframes
    let keyframes = generate_zoom_keyframes(&sessions, data.duration, settings);

    let mut track = TransformTrack::new();
    for kf in keyframes {
        track.add_keyframe(kf);
    }
    track
}

fn generate_zoom_keyframes(
    sessions: &[WorkSession],
    total_duration: f64,
    settings: &SmartZoomSettings,
) -> Vec<TransformKeyframe> {
    if sessions.is_empty() {
        return Vec::new();
    }

    let mut keyframes: Vec<TransformKeyframe> = Vec::new();
    let mut last_session_end: f64 = 0.0;

    for (i, session) in sessions.iter().enumerate() {
        // Zoom-in keyframe (start zooming before session begins)
        let zoom_in_start = (session.start_time - settings.focusing_duration)
            .max(last_session_end + 0.1)
            .max(0.0);

        // Hold at min zoom before zooming in
        keyframes.push(TransformKeyframe::new(
            zoom_in_start,
            settings.min_zoom,
            NormalizedPoint::CENTER,
            settings.zoom_in_easing.clone(),
        ));

        // Zoomed-in keyframe at session start
        keyframes.push(TransformKeyframe::new(
            session.start_time,
            session.zoom,
            session.center,
            settings.move_easing.clone(),
        ));

        // Determine zoom-out/transition
        let hold_end = session.end_time + settings.idle_timeout;

        if let Some(next_session) = sessions.get(i + 1) {
            let time_between = next_session.start_time - session.end_time;

            if time_between < settings.idle_timeout + settings.transition_duration {
                // Direct transition to next session
                let move_start = session.end_time
                    + (time_between * 0.3).min(1.0);

                keyframes.push(TransformKeyframe::new(
                    move_start,
                    session.zoom,
                    session.center,
                    settings.move_easing.clone(),
                ));
            } else {
                // Zoom out, then zoom in to next
                let zoom_out_start = (next_session.start_time
                    - settings.focusing_duration
                    - settings.transition_duration)
                    .max(hold_end);

                // Hold at zoom level
                keyframes.push(TransformKeyframe::new(
                    zoom_out_start,
                    session.zoom,
                    session.center,
                    settings.zoom_out_easing.clone(),
                ));

                // Zoom out
                let zoom_out_end = zoom_out_start + settings.transition_duration;
                keyframes.push(TransformKeyframe::new(
                    zoom_out_end,
                    settings.min_zoom,
                    NormalizedPoint::CENTER,
                    settings.zoom_in_easing.clone(),
                ));
            }
        } else {
            // Final session: zoom out before video ends
            let zoom_out_start = (total_duration - settings.transition_duration - 0.1)
                .max(hold_end);

            keyframes.push(TransformKeyframe::new(
                zoom_out_start,
                session.zoom,
                session.center,
                settings.zoom_out_easing.clone(),
            ));

            keyframes.push(TransformKeyframe::new(
                total_duration - 0.1,
                settings.min_zoom,
                NormalizedPoint::CENTER,
                EasingCurve::Linear,
            ));
        }

        last_session_end = session.end_time;
    }

    // Clamp times to video duration and centers to valid range
    for kf in &mut keyframes {
        kf.time = kf.time.clamp(0.0, total_duration);
        if kf.zoom > 1.0 {
            let half = 0.5 / kf.zoom;
            kf.center = NormalizedPoint::new(
                kf.center.x.clamp(half, 1.0 - half),
                kf.center.y.clamp(half, 1.0 - half),
            );
        }
    }

    optimize_keyframes(&mut keyframes);
    keyframes
}

/// Remove duplicate keyframes that are too close in time
fn optimize_keyframes(keyframes: &mut Vec<TransformKeyframe>) {
    keyframes.sort_by(|a, b| a.time.partial_cmp(&b.time).unwrap());
    keyframes.dedup_by(|b, a| (b.time - a.time).abs() < 0.01);
}

// ============================================================================
// Ripple Generator
// ============================================================================

/// Settings for ripple generation
#[derive(Debug, Clone)]
pub struct RippleSettings {
    pub intensity: f64,
    pub duration: f64,
    pub color: RippleColor,
    pub min_interval: f64,
}

impl Default for RippleSettings {
    fn default() -> Self {
        Self {
            intensity: 0.8,
            duration: 0.4,
            color: RippleColor::LeftClick,
            min_interval: 0.1,
        }
    }
}

/// Generate ripple keyframes from click events
pub fn generate_ripples(clicks: &[ClickEvent], settings: &RippleSettings) -> RippleTrack {
    let mut track = RippleTrack::new();
    let mut last_time: f64 = -1.0;

    for click in clicks {
        if click.click_type != ClickType::LeftDown && click.click_type != ClickType::DoubleClick {
            continue;
        }

        if click.time - last_time < settings.min_interval {
            continue;
        }

        let kf = RippleKeyframe {
            id: Uuid::new_v4(),
            time: click.time,
            position: click.position,
            intensity: settings.intensity,
            duration: settings.duration,
            color: settings.color.clone(),
            easing: EasingCurve::spring_bouncy(),
        };

        track.add_keyframe(kf);
        last_time = click.time;
    }

    track
}

// ============================================================================
// Keystroke Generator
// ============================================================================

/// Settings for keystroke generation
#[derive(Debug, Clone)]
pub struct KeystrokeSettings {
    pub shortcuts_only: bool,
    pub display_duration: f64,
    pub fade_in_duration: f64,
    pub fade_out_duration: f64,
    pub min_interval: f64,
}

impl Default for KeystrokeSettings {
    fn default() -> Self {
        Self {
            shortcuts_only: true,
            display_duration: 1.5,
            fade_in_duration: 0.15,
            fade_out_duration: 0.3,
            min_interval: 0.2,
        }
    }
}

/// Generate keystroke overlay keyframes from keyboard events
pub fn generate_keystrokes(
    events: &[KeyboardEvent],
    settings: &KeystrokeSettings,
) -> KeystrokeTrack {
    let mut track = KeystrokeTrack::new();
    let mut last_time: f64 = -1.0;

    let key_downs: Vec<&KeyboardEvent> = events
        .iter()
        .filter(|e| e.event_type == KeyEventType::KeyDown)
        .collect();

    for event in key_downs {
        // Skip standalone modifiers
        let key_name = match key_display_name(event.key_code, event.character.as_deref()) {
            Some(name) => name,
            None => continue,
        };

        // Shortcuts-only mode: skip regular keys without modifiers
        if settings.shortcuts_only && !event.modifiers.has_modifiers() {
            continue;
        }

        // Auto-repeat filtering
        if event.time - last_time < settings.min_interval {
            continue;
        }

        let mod_symbols = modifier_symbols(&event.modifiers);
        let display_text = format!("{}{}", mod_symbols, key_name);

        let kf = KeystrokeKeyframe {
            id: Uuid::new_v4(),
            time: event.time,
            display_text,
            duration: settings.display_duration,
            fade_in_duration: settings.fade_in_duration,
            fade_out_duration: settings.fade_out_duration,
            position: NormalizedPoint::new(0.5, 0.95),
            easing: EasingCurve::EaseOut,
        };

        track.add_keyframe(kf);
        last_time = event.time;
    }

    track
}

fn modifier_symbols(mods: &Modifiers) -> String {
    let mut s = String::new();
    if mods.control { s.push_str("Ctrl+"); }
    if mods.alt { s.push_str("Alt+"); }
    if mods.shift { s.push_str("Shift+"); }
    if mods.command { s.push_str("Cmd+"); }
    s
}

fn key_display_name(key_code: u16, character: Option<&str>) -> Option<String> {
    // Common named keys (cross-platform key codes vary;
    // these match common virtual key codes)
    let name = match key_code {
        // Standard named keys
        // Enter (Windows VK_RETURN=0x0D, macOS=36)
        0x0D | 36 => "Enter",
        // Tab (Windows VK_TAB=0x09, macOS=48)
        0x09 | 48 => "Tab",
        // Space (Windows VK_SPACE=0x20, macOS=49)
        0x20 | 49 => "Space",
        // Backspace (Windows VK_BACK=0x08, macOS=51)
        0x08 | 51 => "Backspace",
        // Escape (Windows VK_ESCAPE=0x1B, macOS=53)
        0x1B | 53 => "Escape",
        // Delete (Windows VK_DELETE=0x2E, macOS=117)
        0x2E | 117 => "Delete",
        // Left (Windows VK_LEFT=0x25, macOS=123)
        0x25 | 123 => "Left",
        // Right (Windows VK_RIGHT=0x27, macOS=124)
        0x27 | 124 => "Right",
        // Down (Windows VK_DOWN=0x28, macOS=125)
        0x28 | 125 => "Down",
        // Up (Windows VK_UP=0x26, macOS=126)
        0x26 | 126 => "Up",
        // Home (Windows VK_HOME=0x24=36 — conflicts with macOS Enter=36, macOS=115)
        115 => "Home",
        // End (Windows VK_END=0x23=35, macOS=119)
        0x23 | 119 => "End",
        // PageUp (Windows VK_PRIOR=0x21=33, macOS=116)
        0x21 | 116 => "PageUp",
        // PageDown (Windows VK_NEXT=0x22=34, macOS=121)
        0x22 | 121 => "PageDown",
        // Modifier-only key codes (skip these)
        54 | 55 | 56 | 58 | 59 | 60 | 61 | 62 | 63 => return None,
        0xA0..=0xA5 => return None, // Windows modifier VKs
        // Otherwise use character
        _ => {
            return character
                .filter(|c| !c.is_empty())
                .map(|c| c.to_uppercase());
        }
    };
    Some(name.to_string())
}

// ============================================================================
// Cursor Generator (simplified — creates keyframes at stops/direction changes)
// ============================================================================

/// Generate cursor style keyframes from mouse positions
pub fn generate_cursor_keyframes(
    positions: &[(f64, NormalizedPoint)],
    clicks: &[ClickEvent],
) -> CursorTrack {
    let mut track = CursorTrack::new();
    let mut keyframes: Vec<CursorStyleKeyframe> = Vec::new();

    // Create keyframes at click locations (show click cursor effect)
    for click in clicks {
        if click.click_type == ClickType::LeftDown || click.click_type == ClickType::RightDown {
            let mut kf = CursorStyleKeyframe::new(click.time);
            kf.position = Some(click.position);
            kf.scale = 2.0; // Slightly smaller during click
            keyframes.push(kf);

            // Restore after click
            let mut restore = CursorStyleKeyframe::new(click.time + click.duration.max(0.05));
            restore.position = Some(click.position);
            restore.scale = 2.5;
            keyframes.push(restore);
        }
    }

    // Detect stops in movement (velocity drops)
    if positions.len() >= 3 {
        let mut last_kf_time: f64 = -1.0;

        for i in 1..positions.len() - 1 {
            let (t_prev, p_prev) = &positions[i - 1];
            let (t_curr, p_curr) = &positions[i];
            let dt = t_curr - t_prev;
            if dt <= 0.0 { continue; }

            let velocity = p_curr.distance(p_prev) / dt;

            // Detect stop (velocity threshold)
            if velocity < 0.005 && t_curr - last_kf_time > 0.3 {
                let mut kf = CursorStyleKeyframe::new(*t_curr);
                kf.position = Some(*p_curr);
                kf.velocity = Some(velocity);
                keyframes.push(kf);
                last_kf_time = *t_curr;
            }
        }
    }

    keyframes.sort_by(|a, b| a.time.partial_cmp(&b.time).unwrap());
    // Deduplicate very close keyframes
    keyframes.dedup_by(|b, a| (b.time - a.time).abs() < 0.05);

    track.style_keyframes = if keyframes.is_empty() { None } else { Some(keyframes) };
    track
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_typing_sessions() {
        let events = vec![
            KeyboardEvent {
                time: 1.0, event_type: KeyEventType::KeyDown,
                key_code: 0, character: Some("a".into()),
                modifiers: Modifiers::default(),
            },
            KeyboardEvent {
                time: 1.2, event_type: KeyEventType::KeyDown,
                key_code: 0, character: Some("b".into()),
                modifiers: Modifiers::default(),
            },
            KeyboardEvent {
                time: 5.0, event_type: KeyEventType::KeyDown,
                key_code: 0, character: Some("c".into()),
                modifiers: Modifiers::default(),
            },
        ];
        let sessions = detect_typing_sessions(&events);
        assert_eq!(sessions.len(), 2);
        assert!((sessions[0].start - 1.0).abs() < 1e-10);
        assert!((sessions[0].end - 1.2).abs() < 1e-10);
        assert!((sessions[1].start - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_cluster_activities() {
        let activities = vec![
            ActivityEvent { time: 1.0, position: NormalizedPoint::new(0.3, 0.3), activity_type: ActivityType::Click },
            ActivityEvent { time: 1.5, position: NormalizedPoint::new(0.32, 0.31), activity_type: ActivityType::Click },
            ActivityEvent { time: 10.0, position: NormalizedPoint::new(0.8, 0.8), activity_type: ActivityType::Click },
        ];
        let settings = SmartZoomSettings::default();
        let sessions = cluster_activities(&activities, &settings);
        assert_eq!(sessions.len(), 2);
        assert_eq!(sessions[0].activities.len(), 2);
        assert_eq!(sessions[1].activities.len(), 1);
    }

    #[test]
    fn test_calculate_zoom() {
        let mut session = WorkSession {
            start_time: 0.0,
            end_time: 1.0,
            activities: Vec::new(),
            work_area: (0.4, 0.4, 0.1, 0.1),
            center: NormalizedPoint::new(0.45, 0.45),
            zoom: 1.0,
        };
        let settings = SmartZoomSettings::default();
        calculate_session_zoom(&mut session, &settings);
        // coverage 0.4 / area 0.1 = 4.0
        assert!((session.zoom - 4.0).abs() < 1e-10);
    }

    #[test]
    fn test_generate_ripples() {
        let clicks = vec![
            ClickEvent { time: 1.0, position: NormalizedPoint::new(0.3, 0.4), click_type: ClickType::LeftDown, duration: 0.1 },
            ClickEvent { time: 1.02, position: NormalizedPoint::new(0.3, 0.4), click_type: ClickType::LeftDown, duration: 0.1 },
            ClickEvent { time: 2.0, position: NormalizedPoint::new(0.5, 0.5), click_type: ClickType::LeftDown, duration: 0.1 },
        ];
        let settings = RippleSettings::default();
        let track = generate_ripples(&clicks, &settings);
        // Second click is within min_interval of first, should be skipped
        assert_eq!(track.keyframe_count(), 2);
    }

    #[test]
    fn test_generate_keystrokes_shortcuts_only() {
        let events = vec![
            KeyboardEvent {
                time: 1.0, event_type: KeyEventType::KeyDown,
                key_code: 0, character: Some("a".into()),
                modifiers: Modifiers::default(), // no modifiers — should be skipped
            },
            KeyboardEvent {
                time: 2.0, event_type: KeyEventType::KeyDown,
                key_code: 0, character: Some("c".into()),
                modifiers: Modifiers { command: true, ..Default::default() },
            },
        ];
        let settings = KeystrokeSettings { shortcuts_only: true, ..Default::default() };
        let track = generate_keystrokes(&events, &settings);
        assert_eq!(track.keyframe_count(), 1);
        assert!(track.keyframes[0].display_text.contains("Cmd+"));
    }
}
