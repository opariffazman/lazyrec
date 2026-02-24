use std::path::{Path, PathBuf};

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

    /// Save project to a `.lazyrec` package directory.
    ///
    /// Package layout:
    /// ```text
    /// MyProject.lazyrec/
    /// ├── project.json
    /// └── recording/
    ///     ├── recording.mp4
    ///     └── recording_mouse.json
    /// ```
    ///
    /// `video_source` and `mouse_source` are the original file paths to copy into the package.
    /// If they already live inside the package directory, they are not re-copied.
    pub fn save(
        &mut self,
        package_dir: &Path,
        video_source: Option<&Path>,
        mouse_source: Option<&Path>,
    ) -> Result<PathBuf, ProjectError> {
        // Create package directory structure
        let recording_dir = package_dir.join("recording");
        std::fs::create_dir_all(&recording_dir)?;

        // Copy video file into package if provided and not already there
        if let Some(src) = video_source {
            let dst = recording_dir.join(&self.media.video_relative_path);
            if src != dst && src.exists() {
                std::fs::copy(src, &dst)?;
            }
        }

        // Copy mouse data into package if provided and not already there
        if let Some(src) = mouse_source {
            let dst = recording_dir.join(&self.media.mouse_data_relative_path);
            if src != dst && src.exists() {
                std::fs::copy(src, &dst)?;
            }
        }

        // Update modified timestamp
        self.modified_at = chrono_now();

        // Write project.json
        let project_json = serde_json::to_string_pretty(self)
            .map_err(|e| ProjectError::Serialization(e.to_string()))?;
        let project_path = package_dir.join("project.json");
        std::fs::write(&project_path, project_json)?;

        Ok(package_dir.to_path_buf())
    }

    /// Load a project from a `.lazyrec` package directory.
    pub fn load(package_dir: &Path) -> Result<Self, ProjectError> {
        let project_path = package_dir.join("project.json");
        if !project_path.exists() {
            return Err(ProjectError::NotFound(
                format!("project.json not found in {}", package_dir.display()),
            ));
        }

        let json = std::fs::read_to_string(&project_path)?;
        let project: Project = serde_json::from_str(&json)
            .map_err(|e| ProjectError::Serialization(e.to_string()))?;

        Ok(project)
    }

    /// Get the absolute path to the video file within a package directory
    pub fn video_path(&self, package_dir: &Path) -> PathBuf {
        package_dir.join("recording").join(&self.media.video_relative_path)
    }

    /// Get the absolute path to the mouse data file within a package directory
    pub fn mouse_data_path(&self, package_dir: &Path) -> PathBuf {
        package_dir.join("recording").join(&self.media.mouse_data_relative_path)
    }
}

/// Project I/O errors
#[derive(Debug, thiserror::Error)]
pub enum ProjectError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    Serialization(String),
    #[error("Not found: {0}")]
    NotFound(String),
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

#[cfg(test)]
mod tests {
    use super::*;

    fn test_media() -> MediaAsset {
        MediaAsset {
            video_relative_path: "recording.mp4".into(),
            mouse_data_relative_path: "recording_mouse.json".into(),
            pixel_size: Size::new(1920.0, 1080.0),
            frame_rate: 60.0,
            duration: 30.0,
        }
    }

    fn test_capture_meta() -> CaptureMeta {
        CaptureMeta::new(Rect::new(0.0, 0.0, 960.0, 540.0), 2.0)
    }

    #[test]
    fn test_project_new() {
        let project = Project::new("Test".into(), test_media(), test_capture_meta());
        assert_eq!(project.name, "Test");
        assert_eq!(project.version, 1);
        assert_eq!(project.duration(), 30.0);
        assert_eq!(project.total_frames(), 1800);
        assert!(!project.is_window_mode());
    }

    #[test]
    fn test_media_aspect_ratio() {
        let media = test_media();
        let ratio = media.aspect_ratio();
        assert!((ratio - 16.0 / 9.0).abs() < 0.001);
    }

    #[test]
    fn test_media_total_frames() {
        let media = test_media();
        assert_eq!(media.total_frames(), 1800);
    }

    #[test]
    fn test_media_frame_duration() {
        let media = test_media();
        assert!((media.frame_duration() - 1.0 / 60.0).abs() < 0.0001);
    }

    #[test]
    fn test_media_zero_height_aspect() {
        let mut media = test_media();
        media.pixel_size.height = 0.0;
        assert!((media.aspect_ratio() - 16.0 / 9.0).abs() < 0.001);
    }

    #[test]
    fn test_media_zero_fps_frame_duration() {
        let mut media = test_media();
        media.frame_rate = 0.0;
        assert!((media.frame_duration() - 1.0 / 60.0).abs() < 0.001);
    }

    #[test]
    fn test_capture_meta_size_pixel() {
        let meta = test_capture_meta();
        let size = meta.size_pixel();
        assert!((size.width - 1920.0).abs() < 0.001);
        assert!((size.height - 1080.0).abs() < 0.001);
    }

    #[test]
    fn test_project_save_load_roundtrip() {
        let dir = std::env::temp_dir().join(format!("lazyrec_test_{}", uuid::Uuid::new_v4()));
        let mut project = Project::new("Roundtrip".into(), test_media(), test_capture_meta());

        project.save(&dir, None, None).unwrap();
        let loaded = Project::load(&dir).unwrap();

        assert_eq!(loaded.name, "Roundtrip");
        assert_eq!(loaded.duration(), 30.0);
        assert_eq!(loaded.media.frame_rate, 60.0);

        // Cleanup
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_project_load_not_found() {
        let result = Project::load(std::path::Path::new("/nonexistent/path.lazyrec"));
        assert!(result.is_err());
    }

    #[test]
    fn test_project_video_path() {
        let project = Project::new("Test".into(), test_media(), test_capture_meta());
        let pkg = PathBuf::from("/tmp/test.lazyrec");
        assert_eq!(project.video_path(&pkg), pkg.join("recording").join("recording.mp4"));
    }

    #[test]
    fn test_project_mouse_data_path() {
        let project = Project::new("Test".into(), test_media(), test_capture_meta());
        let pkg = PathBuf::from("/tmp/test.lazyrec");
        assert_eq!(project.mouse_data_path(&pkg), pkg.join("recording").join("recording_mouse.json"));
    }

    #[test]
    fn test_output_resolution_sizes() {
        let source = Size::new(2560.0, 1440.0);
        assert_eq!(OutputResolution::Original.size(&source).width, 2560.0);
        assert_eq!(OutputResolution::Uhd4k.size(&source).width, 3840.0);
        assert_eq!(OutputResolution::Fhd1080.size(&source).height, 1080.0);
        assert_eq!(OutputResolution::Hd720.size(&source).width, 1280.0);
        let custom = OutputResolution::Custom { width: 800, height: 600 };
        assert_eq!(custom.size(&source).width, 800.0);
    }

    #[test]
    fn test_output_frame_rate_value() {
        assert_eq!(OutputFrameRate::Original.value(60.0), 60.0);
        assert_eq!(OutputFrameRate::Fixed { fps: 30 }.value(60.0), 30.0);
    }

    #[test]
    fn test_video_codec() {
        assert_eq!(VideoCodec::H264.file_extension(), "mp4");
        assert_eq!(VideoCodec::H265.display_name(), "H.265 (HEVC)");
    }

    #[test]
    fn test_export_quality_bit_rate() {
        let br = ExportQuality::High.bit_rate(1920.0, 1080.0);
        assert!(br > 0);
        assert!(ExportQuality::Original.bit_rate_multiplier() > ExportQuality::High.bit_rate_multiplier());
    }

    #[test]
    fn test_render_settings_default() {
        let s = RenderSettings::default();
        assert!(!s.background_enabled);
        assert!((s.corner_radius - 22.0).abs() < 0.001);
        assert_eq!(s.codec, VideoCodec::H265);
    }

    #[test]
    fn test_key_modifiers_to_strings() {
        let mods = KeyModifiers {
            command: true,
            shift: true,
            alt: false,
            control: false,
            function_key: false,
            caps_lock: false,
        };
        let strings = mods.to_strings();
        assert_eq!(strings, vec!["command", "shift"]);
    }

    #[test]
    fn test_project_serde_roundtrip() {
        let project = Project::new("Serde".into(), test_media(), test_capture_meta());
        let json = serde_json::to_string(&project).unwrap();
        let loaded: Project = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded.name, "Serde");
        assert_eq!(loaded.duration(), 30.0);
    }
}
