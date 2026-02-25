//! Video encoding abstraction via FFmpeg (ffmpeg-next crate).
//! Trait-based design allows future alternative backends.

use std::path::PathBuf;

use super::project::{ExportQuality, VideoCodec};

/// Whether the encoder is used for live recording or offline export.
/// Recording needs speed (ultrafast); export can trade speed for quality.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncoderPurpose {
    Recording,
    Export,
}

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
    /// Whether this encoder is for live recording or offline export
    pub purpose: EncoderPurpose,
}

impl EncoderConfig {
    pub fn new(width: u32, height: u32, output_path: PathBuf) -> Self {
        Self {
            width,
            height,
            frame_rate: 60,
            codec: VideoCodec::H264,
            quality: ExportQuality::High,
            output_path,
            keyframe_interval: 120,
            purpose: EncoderPurpose::Recording,
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

/// FFmpeg-based video encoder using ffmpeg-next crate.
/// Encodes BGRA frames to H.264 or H.265 MP4 output.
#[cfg(feature = "ffmpeg")]
pub mod ffmpeg_encoder {
    use super::*;
    use ffmpeg_next as ffmpeg;
    use ffmpeg::codec;
    use ffmpeg::format;
    use ffmpeg::software::scaling;
    use ffmpeg::util::frame::video::Video as FfmpegFrame;

    /// Wrapper to make scaling::Context Send-safe.
    /// SwsContext is safe to use from one thread at a time (our usage pattern).
    struct SendScaler(scaling::Context);
    // SAFETY: We only access the scaler from a single thread at a time.
    unsafe impl Send for SendScaler {}

    impl std::ops::Deref for SendScaler {
        type Target = scaling::Context;
        fn deref(&self) -> &Self::Target { &self.0 }
    }
    impl std::ops::DerefMut for SendScaler {
        fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
    }

    pub struct FfmpegEncoder {
        config: EncoderConfig,
        encoding: bool,
        frame_count: u64,
        output_ctx: Option<format::context::Output>,
        encoder: Option<codec::encoder::video::Encoder>,
        scaler: Option<SendScaler>,
        stream_index: usize,
        time_base: ffmpeg::Rational,
    }

    impl FfmpegEncoder {
        pub fn new(config: EncoderConfig) -> Result<Self, EncoderError> {
            ffmpeg::init().map_err(|e| EncoderError::Ffmpeg(format!("FFmpeg init: {e}")))?;

            Ok(Self {
                config,
                encoding: false,
                frame_count: 0,
                output_ctx: None,
                encoder: None,
                scaler: None,
                stream_index: 0,
                time_base: ffmpeg::Rational::new(1, 60),
            })
        }
    }

    impl VideoEncoder for FfmpegEncoder {
        fn start(&mut self) -> Result<(), EncoderError> {
            if self.encoding {
                return Err(EncoderError::AlreadyStarted);
            }

            let path = &self.config.output_path;
            let mut output_ctx = format::output(path)
                .map_err(|e| EncoderError::Ffmpeg(format!("Open output: {e}")))?;

            let codec_id = match self.config.codec {
                VideoCodec::H264 => codec::Id::H264,
                VideoCodec::H265 => codec::Id::HEVC,
            };

            let codec = codec::encoder::find(codec_id)
                .ok_or_else(|| EncoderError::Ffmpeg(format!("Codec {:?} not found", codec_id)))?;

            // Check global header flag before add_stream borrows output_ctx
            let needs_global_header = output_ctx.format().flags().contains(format::Flags::GLOBAL_HEADER);

            let mut stream = output_ctx.add_stream(codec)
                .map_err(|e| EncoderError::Ffmpeg(format!("Add stream: {e}")))?;
            self.stream_index = stream.index();

            let time_base = ffmpeg::Rational::new(1, self.config.frame_rate as i32);
            self.time_base = time_base;

            let mut encoder_ctx = codec::context::Context::new_with_codec(codec)
                .encoder()
                .video()
                .map_err(|e| EncoderError::Ffmpeg(format!("Encoder context: {e}")))?;

            encoder_ctx.set_width(self.config.width);
            encoder_ctx.set_height(self.config.height);
            encoder_ctx.set_format(ffmpeg::format::Pixel::YUV420P);
            encoder_ctx.set_time_base(time_base);
            encoder_ctx.set_bit_rate(self.config.bit_rate() as usize);
            encoder_ctx.set_gop(self.config.keyframe_interval);
            encoder_ctx.set_threading(codec::threading::Config::count(4));

            if needs_global_header {
                encoder_ctx.set_flags(codec::Flags::GLOBAL_HEADER);
            }

            // Use ultrafast preset for both recording and export — speed is critical.
            // At CRF 23 the quality difference between ultrafast and fast is negligible
            // but ultrafast is 3-5x faster (crucial at 3440x1440).
            let preset = "ultrafast";

            let mut opts = ffmpeg::Dictionary::new();
            opts.set("preset", preset);
            // Use CRF (constant quality) instead of CBR for better quality/speed
            opts.set("crf", "23");

            let encoder = encoder_ctx.open_as_with(codec, opts)
                .map_err(|e| EncoderError::Ffmpeg(format!("Open encoder: {e}")))?;

            stream.set_parameters(&encoder);

            output_ctx.write_header()
                .map_err(|e| EncoderError::Ffmpeg(format!("Write header: {e}")))?;

            // BGRA -> YUV420P scaler
            let scaler = scaling::Context::get(
                ffmpeg::format::Pixel::BGRA,
                self.config.width,
                self.config.height,
                ffmpeg::format::Pixel::YUV420P,
                self.config.width,
                self.config.height,
                scaling::Flags::FAST_BILINEAR,
            ).map_err(|e| EncoderError::Ffmpeg(format!("Scaler init: {e}")))?;

            self.output_ctx = Some(output_ctx);
            self.encoder = Some(encoder);
            self.scaler = Some(SendScaler(scaler));
            self.encoding = true;
            self.frame_count = 0;

            Ok(())
        }

        fn append_frame(&mut self, frame: &VideoFrame) -> Result<(), EncoderError> {
            if !self.encoding {
                return Err(EncoderError::NotStarted);
            }

            let encoder = self.encoder.as_mut().unwrap();
            let scaler = self.scaler.as_mut().unwrap();
            let output_ctx = self.output_ctx.as_mut().unwrap();

            // Create BGRA input frame
            let mut bgra_frame = FfmpegFrame::new(
                ffmpeg::format::Pixel::BGRA,
                frame.width,
                frame.height,
            );
            bgra_frame.data_mut(0)[..frame.data.len()].copy_from_slice(&frame.data);

            // Convert BGRA -> YUV420P
            let mut yuv_frame = FfmpegFrame::empty();
            scaler.run(&bgra_frame, &mut yuv_frame)
                .map_err(|e| EncoderError::Ffmpeg(format!("Scale frame: {e}")))?;

            let pts = self.frame_count as i64;
            yuv_frame.set_pts(Some(pts));

            // Send frame to encoder
            encoder.send_frame(&yuv_frame)
                .map_err(|e| EncoderError::Ffmpeg(format!("Send frame: {e}")))?;

            // Receive and write encoded packets
            let mut packet = ffmpeg::Packet::empty();
            while encoder.receive_packet(&mut packet).is_ok() {
                packet.set_stream(self.stream_index);
                packet.rescale_ts(self.time_base, output_ctx.stream(self.stream_index).unwrap().time_base());
                packet.write_interleaved(output_ctx)
                    .map_err(|e| EncoderError::Ffmpeg(format!("Write packet: {e}")))?;
            }

            self.frame_count += 1;
            Ok(())
        }

        fn finish(&mut self) -> Result<PathBuf, EncoderError> {
            if !self.encoding {
                return Err(EncoderError::NotStarted);
            }

            let encoder = self.encoder.as_mut().unwrap();
            let output_ctx = self.output_ctx.as_mut().unwrap();

            // Flush encoder
            encoder.send_eof()
                .map_err(|e| EncoderError::Ffmpeg(format!("Send EOF: {e}")))?;

            let mut packet = ffmpeg::Packet::empty();
            while encoder.receive_packet(&mut packet).is_ok() {
                packet.set_stream(self.stream_index);
                packet.rescale_ts(self.time_base, output_ctx.stream(self.stream_index).unwrap().time_base());
                packet.write_interleaved(output_ctx)
                    .map_err(|e| EncoderError::Ffmpeg(format!("Write packet: {e}")))?;
            }

            output_ctx.write_trailer()
                .map_err(|e| EncoderError::Ffmpeg(format!("Write trailer: {e}")))?;

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
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> EncoderConfig {
        EncoderConfig::new(1920, 1080, PathBuf::from("/tmp/test_output.mp4"))
    }

    #[test]
    fn test_encoder_config_defaults() {
        let cfg = test_config();
        assert_eq!(cfg.width, 1920);
        assert_eq!(cfg.height, 1080);
        assert_eq!(cfg.frame_rate, 60);
        assert_eq!(cfg.keyframe_interval, 120);
        assert!(matches!(cfg.codec, VideoCodec::H264));
        assert!(matches!(cfg.quality, ExportQuality::High));
    }

    #[test]
    fn test_encoder_config_bit_rate() {
        let cfg = test_config();
        let br = cfg.bit_rate();
        assert!(br > 0, "Bit rate should be positive");
    }

    #[test]
    fn test_stub_encoder_lifecycle() {
        let cfg = test_config();
        let mut enc = StubEncoder::new(cfg);
        assert!(!enc.is_encoding());
        assert_eq!(enc.frames_encoded(), 0);

        enc.start().unwrap();
        assert!(enc.is_encoding());

        let frame = VideoFrame {
            data: vec![0u8; 1920 * 1080 * 4],
            width: 1920,
            height: 1080,
            stride: 1920 * 4,
            pts: 0.0,
        };
        enc.append_frame(&frame).unwrap();
        enc.append_frame(&frame).unwrap();
        assert_eq!(enc.frames_encoded(), 2);

        let path = enc.finish().unwrap();
        assert_eq!(path, PathBuf::from("/tmp/test_output.mp4"));
        assert!(!enc.is_encoding());
    }

    #[test]
    fn test_stub_encoder_double_start_errors() {
        let mut enc = StubEncoder::new(test_config());
        enc.start().unwrap();
        match enc.start() {
            Err(EncoderError::AlreadyStarted) => {}
            other => panic!("Expected AlreadyStarted, got {:?}", other),
        }
    }

    #[test]
    fn test_stub_encoder_append_without_start_errors() {
        let mut enc = StubEncoder::new(test_config());
        let frame = VideoFrame {
            data: vec![],
            width: 0,
            height: 0,
            stride: 0,
            pts: 0.0,
        };
        match enc.append_frame(&frame) {
            Err(EncoderError::NotStarted) => {}
            other => panic!("Expected NotStarted, got {:?}", other),
        }
    }

    #[test]
    fn test_stub_encoder_finish_without_start_errors() {
        let mut enc = StubEncoder::new(test_config());
        match enc.finish() {
            Err(EncoderError::NotStarted) => {}
            other => panic!("Expected NotStarted, got {:?}", other),
        }
    }

    #[test]
    fn test_stub_encoder_start_resets_frame_count() {
        let mut enc = StubEncoder::new(test_config());
        enc.start().unwrap();
        let frame = VideoFrame {
            data: vec![],
            width: 0,
            height: 0,
            stride: 0,
            pts: 0.0,
        };
        enc.append_frame(&frame).unwrap();
        assert_eq!(enc.frames_encoded(), 1);
        enc.finish().unwrap();
        enc.start().unwrap();
        assert_eq!(enc.frames_encoded(), 0);
    }
}

/// Create the video encoder.
/// Returns FFmpeg encoder when the `ffmpeg` feature is enabled,
/// otherwise falls back to the stub encoder.
pub fn create_encoder(config: EncoderConfig) -> Box<dyn VideoEncoder> {
    #[cfg(feature = "ffmpeg")]
    {
        match ffmpeg_encoder::FfmpegEncoder::new(config.clone()) {
            Ok(enc) => return Box::new(enc),
            Err(e) => {
                log::warn!("FFmpeg encoder init failed, falling back to stub: {e}");
            }
        }
    }

    Box::new(StubEncoder::new(config))
}
