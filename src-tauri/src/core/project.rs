use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::timeline::Timeline;

/// LazyRec project file.
/// Contains recorded media and timeline editing data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: Uuid,
    pub version: u32,
    pub name: String,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(rename = "modifiedAt")]
    pub modified_at: String,
    pub media: MediaAsset,
    #[serde(rename = "captureMeta")]
    pub capture_meta: CaptureMeta,
    pub timeline: Timeline,
    #[serde(rename = "renderSettings")]
    pub render_settings: RenderSettings,
}

impl Project {
    pub fn new(name: String, media: MediaAsset, capture_meta: CaptureMeta) -> Self {
        let now = chrono_now();
        Self {
            id: Uuid::new_v4(),
            version: 1,
            name,
            created_at: now.clone(),
            modified_at: now,
            media: media.clone(),
            capture_meta,
            timeline: Timeline::with_default_tracks(media.duration),
            render_settings: RenderSettings::default(),
        }
    }

    pub fn duration(&self) -> f64 {
        self.media.duration
    }

    pub fn total_frames(&self) -> u64 {
        (self.media.duration * self.media.frame_rate) as u64
    }

    pub fn is_window_mode(&self) -> bool {
        self.render_settings.background_enabled
    }

    /// Package extension for project directories
    pub const PACKAGE_EXTENSION: &'static str = "lazyrec";
}

fn chrono_now() -> String {
    // ISO 8601 timestamp
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format!("{now}")
}

/// Media asset information.
/// References to the original video and mouse data files.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaAsset {
    /// Relative path within the package
    #[serde(rename = "videoPath")]
    pub video_relative_path: String,
    /// Relative path within the package
    #[serde(rename = "mouseDataPath")]
    pub mouse_data_relative_path: String,
    /// Original video resolution (pixels)
    #[serde(rename = "pixelSize")]
    pub pixel_size: Size,
    /// Original frame rate
    #[serde(rename = "frameRate")]
    pub frame_rate: f64,
    /// Total duration (seconds)
    pub duration: f64,
}

impl MediaAsset {
    pub fn aspect_ratio(&self) -> f64 {
        if self.pixel_size.height <= 0.0 {
            return 16.0 / 9.0;
        }
        self.pixel_size.width / self.pixel_size.height
    }

    pub fn total_frames(&self) -> u64 {
        (self.duration * self.frame_rate) as u64
    }

    pub fn frame_duration(&self) -> f64 {
        if self.frame_rate <= 0.0 {
            return 1.0 / 60.0;
        }
        1.0 / self.frame_rate
    }
}

/// Simple size struct (replaces CGSize)
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Size {
    pub width: f64,
    pub height: f64,
}

impl Size {
    pub fn new(width: f64, height: f64) -> Self {
        Self { width, height }
    }
}

/// Capture metadata.
/// Display and coordinate snapshot recorded at capture time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureMeta {
    /// Capture bounds in platform points
    #[serde(rename = "boundsPt")]
    pub bounds_pt: Rect,
    /// Scale factor (HiDPI: 2.0, standard: 1.0)
    #[serde(rename = "scaleFactor")]
    pub scale_factor: f64,
}

impl CaptureMeta {
    pub fn new(bounds_pt: Rect, scale_factor: f64) -> Self {
        Self { bounds_pt, scale_factor }
    }

    pub fn size_pixel(&self) -> Size {
        Size {
            width: self.bounds_pt.width * self.scale_factor,
            height: self.bounds_pt.height * self.scale_factor,
        }
    }
}

/// Simple rect struct (replaces CGRect)
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl Rect {
    pub fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self { x, y, width, height }
    }
}

/// Rendering settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderSettings {
    #[serde(rename = "outputResolution")]
    pub output_resolution: OutputResolution,
    #[serde(rename = "outputFrameRate")]
    pub output_frame_rate: OutputFrameRate,
    pub codec: VideoCodec,
    pub quality: ExportQuality,
    #[serde(rename = "backgroundEnabled")]
    pub background_enabled: bool,
    #[serde(rename = "cornerRadius")]
    pub corner_radius: f64,
    #[serde(rename = "shadowRadius")]
    pub shadow_radius: f64,
    #[serde(rename = "shadowOpacity")]
    pub shadow_opacity: f64,
    pub padding: f64,
    #[serde(rename = "windowInset")]
    pub window_inset: f64,
}

impl Default for RenderSettings {
    fn default() -> Self {
        Self {
            output_resolution: OutputResolution::Original,
            output_frame_rate: OutputFrameRate::Original,
            codec: VideoCodec::H265,
            quality: ExportQuality::High,
            background_enabled: false,
            corner_radius: 22.0,
            shadow_radius: 40.0,
            shadow_opacity: 0.7,
            padding: 40.0,
            window_inset: 12.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum OutputResolution {
    #[serde(rename = "original")]
    Original,
    #[serde(rename = "uhd4k")]
    Uhd4k,
    #[serde(rename = "qhd1440")]
    Qhd1440,
    #[serde(rename = "fhd1080")]
    Fhd1080,
    #[serde(rename = "hd720")]
    Hd720,
    #[serde(rename = "custom")]
    Custom { width: u32, height: u32 },
}

impl OutputResolution {
    pub fn size(&self, source: &Size) -> Size {
        match self {
            Self::Original => *source,
            Self::Uhd4k => Size::new(3840.0, 2160.0),
            Self::Qhd1440 => Size::new(2560.0, 1440.0),
            Self::Fhd1080 => Size::new(1920.0, 1080.0),
            Self::Hd720 => Size::new(1280.0, 720.0),
            Self::Custom { width, height } => Size::new(*width as f64, *height as f64),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum OutputFrameRate {
    #[serde(rename = "original")]
    Original,
    #[serde(rename = "fixed")]
    Fixed { fps: u32 },
}

impl OutputFrameRate {
    pub fn value(&self, source_fps: f64) -> f64 {
        match self {
            Self::Original => source_fps,
            Self::Fixed { fps } => *fps as f64,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum VideoCodec {
    H264,
    H265,
}

impl VideoCodec {
    pub fn file_extension(&self) -> &str {
        "mp4"
    }

    pub fn display_name(&self) -> &str {
        match self {
            Self::H264 => "H.264",
            Self::H265 => "H.265 (HEVC)",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ExportQuality {
    Low,
    Medium,
    High,
    Original,
}

impl ExportQuality {
    pub fn bit_rate_multiplier(&self) -> f64 {
        match self {
            Self::Low => 2.0,
            Self::Medium => 4.0,
            Self::High => 8.0,
            Self::Original => 12.0,
        }
    }

    pub fn bit_rate(&self, width: f64, height: f64) -> u64 {
        let pixels = width * height;
        (pixels * self.bit_rate_multiplier()) as u64
    }
}

// MARK: - Mouse Data Format (polyrecorder v4 compatible)

/// Mouse movement event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MouseMoveEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    #[serde(rename = "processTimeMs")]
    pub process_time_ms: i64,
    #[serde(rename = "unixTimeMs")]
    pub unix_time_ms: i64,
    pub x: f64,
    pub y: f64,
    #[serde(rename = "cursorId")]
    pub cursor_id: Option<String>,
    #[serde(rename = "activeModifiers")]
    pub active_modifiers: Vec<String>,
    pub button: Option<String>,
}

/// Mouse click event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MouseClickEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    #[serde(rename = "processTimeMs")]
    pub process_time_ms: i64,
    #[serde(rename = "unixTimeMs")]
    pub unix_time_ms: i64,
    pub x: f64,
    pub y: f64,
    pub button: String,
    #[serde(rename = "cursorId")]
    pub cursor_id: Option<String>,
    #[serde(rename = "activeModifiers")]
    pub active_modifiers: Vec<String>,
}

/// Keystroke event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeystrokeEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    #[serde(rename = "processTimeMs")]
    pub process_time_ms: i64,
    #[serde(rename = "unixTimeMs")]
    pub unix_time_ms: i64,
    pub character: Option<String>,
    #[serde(rename = "isARepeat")]
    pub is_a_repeat: bool,
    #[serde(rename = "activeModifiers")]
    pub active_modifiers: Vec<String>,
}

/// Key modifiers
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct KeyModifiers {
    pub command: bool,
    pub shift: bool,
    pub alt: bool,
    pub control: bool,
    #[serde(rename = "function")]
    pub function_key: bool,
    pub caps_lock: bool,
}

impl KeyModifiers {
    pub fn to_strings(&self) -> Vec<String> {
        let mut result = Vec::new();
        if self.command { result.push("command".into()); }
        if self.shift { result.push("shift".into()); }
        if self.alt { result.push("alt".into()); }
        if self.control { result.push("control".into()); }
        if self.function_key { result.push("function".into()); }
        if self.caps_lock { result.push("capsLock".into()); }
        result
    }
}
