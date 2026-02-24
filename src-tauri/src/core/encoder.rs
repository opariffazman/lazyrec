//! Video encoding abstraction via FFmpeg (ffmpeg-next crate).
//! Trait-based design allows future alternative backends.

use std::path::PathBuf;

use super::project::{ExportQuality, VideoCodec};

/// Video encoder configuration
#[derive(Debug, Clone)]
pub struct EncoderConfig {
    pub width: u32,
    pub height: u32,
    pub frame_rate: u32,
    pub codec: VideoCodec,
    pub quality: ExportQuality,
    pub output_path: PathBuf,
    /// Key frame interval (GOP size)
    pub keyframe_interval: u32,
}

impl EncoderConfig {
    pub fn new(width: u32, height: u32, output_path: PathBuf) -> Self {
        Self {
            width,
            height,
            frame_rate: 60,
            codec: VideoCodec::H265,
            quality: ExportQuality::High,
            output_path,
            keyframe_interval: 120,
        }
    }

    pub fn bit_rate(&self) -> u64 {
        self.quality.bit_rate(self.width as f64, self.height as f64)
    }
}

/// A raw video frame to encode
pub struct VideoFrame {
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub stride: u32,
    /// Presentation timestamp in seconds
    pub pts: f64,
}

/// Encoder error types
#[derive(Debug, thiserror::Error)]
pub enum EncoderError {
    #[error("Encoder already started")]
    AlreadyStarted,
    #[error("Encoder not started")]
    NotStarted,
    #[error("FFmpeg error: {0}")]
    Ffmpeg(String),
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Video encoder abstraction trait.
/// Currently implemented as a stub; full FFmpeg implementation
/// requires the ffmpeg-next crate dependency.
pub trait VideoEncoder: Send {
    /// Start encoding to the configured output file
    fn start(&mut self) -> Result<(), EncoderError>;

    /// Append a video frame (must be called in PTS order)
    fn append_frame(&mut self, frame: &VideoFrame) -> Result<(), EncoderError>;

    /// Finalize encoding and flush remaining frames
    fn finish(&mut self) -> Result<PathBuf, EncoderError>;

    /// Check if encoder is actively encoding
    fn is_encoding(&self) -> bool;

    /// Get the number of frames encoded so far
    fn frames_encoded(&self) -> u64;
}

/// Stub encoder for development (writes no actual video)
pub struct StubEncoder {
    config: EncoderConfig,
    encoding: bool,
    frame_count: u64,
}

impl StubEncoder {
    pub fn new(config: EncoderConfig) -> Self {
        Self {
            config,
            encoding: false,
            frame_count: 0,
        }
    }
}

impl VideoEncoder for StubEncoder {
    fn start(&mut self) -> Result<(), EncoderError> {
        if self.encoding {
            return Err(EncoderError::AlreadyStarted);
        }
        self.encoding = true;
        self.frame_count = 0;
        Ok(())
    }

    fn append_frame(&mut self, _frame: &VideoFrame) -> Result<(), EncoderError> {
        if !self.encoding {
            return Err(EncoderError::NotStarted);
        }
        self.frame_count += 1;
        Ok(())
    }

    fn finish(&mut self) -> Result<PathBuf, EncoderError> {
        if !self.encoding {
            return Err(EncoderError::NotStarted);
        }
        self.encoding = false;
        Ok(self.config.output_path.clone())
    }

    fn is_encoding(&self) -> bool {
        self.encoding
    }

    fn frames_encoded(&self) -> u64 {
        self.frame_count
    }
}

/// Create the video encoder.
/// Currently returns a stub; will be replaced with FFmpeg backend
/// when the ffmpeg-next dependency is added.
pub fn create_encoder(config: EncoderConfig) -> Box<dyn VideoEncoder> {
    // TODO: Replace with FfmpegEncoder when ffmpeg-next is integrated
    Box::new(StubEncoder::new(config))
}
