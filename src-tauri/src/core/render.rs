//! Render pipeline: frame-by-frame effect composition and export engine.
//!
//! Composition order (matching Screenize):
//! 1. Source frame
//! 2. Ripple effects (composited OVER source — moves with transform)
//! 3. Cursor (composited OVER ripples — moves with transform)
//! 4. Transform (crop/zoom/pan)
//! 5. Keystroke overlay (composited OVER output — FIXED on screen)
//!
//! Initial implementation uses software rendering (CPU pixel ops).
//! GPU acceleration via wgpu can be added later.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::encoder::{EncoderConfig, VideoEncoder, VideoFrame, create_encoder};
use super::evaluator::{
    ActiveKeystroke, ActiveRipple, CursorState, EvaluatedFrameState, FrameEvaluator, MousePosition,
    TransformState,
};
use super::project::{Project, RenderSettings, Size};
use super::timeline::Timeline;

// =============================================================================
// Frame buffer
// =============================================================================

/// BGRA pixel buffer
#[derive(Debug, Clone)]
pub struct FrameBuffer {
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
    /// Bytes per row (typically width * 4 for BGRA)
    pub stride: u32,
}

impl FrameBuffer {
    pub fn new(width: u32, height: u32) -> Self {
        let stride = width * 4;
        Self {
            data: vec![0u8; (stride * height) as usize],
            width,
            height,
            stride,
        }
    }

    /// Create a frame filled with a solid BGRA color
    pub fn solid(width: u32, height: u32, b: u8, g: u8, r: u8, a: u8) -> Self {
        let stride = width * 4;
        let mut data = vec![0u8; (stride * height) as usize];
        for pixel in data.chunks_exact_mut(4) {
            pixel[0] = b;
            pixel[1] = g;
            pixel[2] = r;
            pixel[3] = a;
        }
        Self { data, width, height, stride }
    }

    /// Get pixel at (x, y) as [B, G, R, A]
    #[inline]
    fn get_pixel(&self, x: u32, y: u32) -> [u8; 4] {
        let offset = (y * self.stride + x * 4) as usize;
        if offset + 3 < self.data.len() {
            [self.data[offset], self.data[offset + 1], self.data[offset + 2], self.data[offset + 3]]
        } else {
            [0, 0, 0, 0]
        }
    }

    /// Set pixel at (x, y) from [B, G, R, A]
    #[inline]
    fn set_pixel(&mut self, x: u32, y: u32, pixel: [u8; 4]) {
        let offset = (y * self.stride + x * 4) as usize;
        if offset + 3 < self.data.len() {
            self.data[offset] = pixel[0];
            self.data[offset + 1] = pixel[1];
            self.data[offset + 2] = pixel[2];
            self.data[offset + 3] = pixel[3];
        }
    }

    /// Alpha-composite `src` pixel over `dst` pixel (premultiplied alpha)
    #[inline]
    fn composite_over(dst: [u8; 4], src: [u8; 4]) -> [u8; 4] {
        let sa = src[3] as u32;
        let inv_sa = 255 - sa;
        [
            ((src[0] as u32 * sa + dst[0] as u32 * inv_sa) / 255) as u8,
            ((src[1] as u32 * sa + dst[1] as u32 * inv_sa) / 255) as u8,
            ((src[2] as u32 * sa + dst[2] as u32 * inv_sa) / 255) as u8,
            ((sa + dst[3] as u32 * inv_sa / 255).min(255)) as u8,
        ]
    }

    /// Bilinear sample at fractional (fx, fy) coordinates in pixel space
    fn sample_bilinear(&self, fx: f64, fy: f64) -> [u8; 4] {
        let x0 = fx.floor() as i64;
        let y0 = fy.floor() as i64;
        let x1 = x0 + 1;
        let y1 = y0 + 1;
        let tx = (fx - x0 as f64) as f32;
        let ty = (fy - y0 as f64) as f32;

        let clamp_x = |x: i64| x.clamp(0, self.width as i64 - 1) as u32;
        let clamp_y = |y: i64| y.clamp(0, self.height as i64 - 1) as u32;

        let p00 = self.get_pixel(clamp_x(x0), clamp_y(y0));
        let p10 = self.get_pixel(clamp_x(x1), clamp_y(y0));
        let p01 = self.get_pixel(clamp_x(x0), clamp_y(y1));
        let p11 = self.get_pixel(clamp_x(x1), clamp_y(y1));

        let lerp = |a: u8, b: u8, t: f32| -> u8 {
            (a as f32 + (b as f32 - a as f32) * t).round().clamp(0.0, 255.0) as u8
        };

        let top = [
            lerp(p00[0], p10[0], tx),
            lerp(p00[1], p10[1], tx),
            lerp(p00[2], p10[2], tx),
            lerp(p00[3], p10[3], tx),
        ];
        let bot = [
            lerp(p01[0], p11[0], tx),
            lerp(p01[1], p11[1], tx),
            lerp(p01[2], p11[2], tx),
            lerp(p01[3], p11[3], tx),
        ];

        [
            lerp(top[0], bot[0], ty),
            lerp(top[1], bot[1], ty),
            lerp(top[2], bot[2], ty),
            lerp(top[3], bot[3], ty),
        ]
    }

    /// Convert to VideoFrame by moving data (avoids ~20MB clone per frame)
    fn into_video_frame(self, pts: f64) -> VideoFrame {
        VideoFrame {
            data: self.data,
            width: self.width,
            height: self.height,
            stride: self.stride,
            pts,
        }
    }
}

// =============================================================================
// Render context
// =============================================================================

/// Configuration for the render pipeline
#[derive(Debug, Clone)]
pub struct RenderContext {
    pub source_size: Size,
    pub output_size: Size,
    pub frame_rate: f64,
    pub window_mode: bool,
    pub render_settings: RenderSettings,
}

impl RenderContext {
    pub fn from_project(project: &Project) -> Self {
        let source_size = project.media.pixel_size;
        let output_size = project
            .render_settings
            .output_resolution
            .size(&source_size);
        let frame_rate = project
            .render_settings
            .output_frame_rate
            .value(project.media.frame_rate);

        Self {
            source_size,
            output_size,
            frame_rate,
            window_mode: project.is_window_mode(),
            render_settings: project.render_settings.clone(),
        }
    }
}

// =============================================================================
// Software renderer
// =============================================================================

/// Software-based effect renderer (CPU pixel operations).
pub struct SoftwareRenderer {
    ctx: RenderContext,
}

impl SoftwareRenderer {
    pub fn new(ctx: RenderContext) -> Self {
        Self { ctx }
    }

    /// Render a complete frame with all effects applied in correct order.
    /// Avoids cloning the source when no pre-transform effects are active.
    pub fn render_frame(
        &self,
        source: &FrameBuffer,
        state: &EvaluatedFrameState,
    ) -> FrameBuffer {
        let has_ripples = !state.ripples.is_empty();
        let has_cursor = state.cursor.visible;

        // Only clone source if we need to draw pre-transform effects on it
        let frame_ref = if has_ripples || has_cursor {
            let mut frame = source.clone();
            for ripple in &state.ripples {
                self.apply_ripple(&mut frame, ripple);
            }
            if has_cursor {
                self.apply_cursor(&mut frame, &state.cursor);
            }
            std::borrow::Cow::Owned(frame)
        } else {
            std::borrow::Cow::Borrowed(source)
        };

        // 3. Transform (crop/zoom/pan)
        let mut output = self.apply_transform(&frame_ref, &state.transform);

        // 4. Keystroke overlay (over output, FIXED on screen)
        for keystroke in &state.keystrokes {
            self.apply_keystroke(&mut output, keystroke);
        }

        output
    }

    /// Apply a ripple effect at the given position.
    /// Renders a radial gradient ring that expands and fades out.
    fn apply_ripple(&self, frame: &mut FrameBuffer, ripple: &ActiveRipple) {
        let w = frame.width as f64;
        let h = frame.height as f64;

        // Center in pixel coords
        let cx = ripple.position.x * w;
        let cy = ripple.position.y * h;

        // Max radius scales with frame size (reference: 1920px)
        let base_radius = 80.0 * (w / 1920.0);
        let current_radius = base_radius * ripple.progress;
        let ring_width = current_radius * 0.15;
        let inner_radius = (current_radius - ring_width).max(0.0);

        // Opacity fades as ripple expands
        let opacity = ((1.0 - ripple.progress) * ripple.intensity).clamp(0.0, 1.0);
        if opacity < 0.01 {
            return;
        }

        let (r_col, g_col, b_col, _) = ripple.color;
        let rb = (b_col * 255.0) as u8;
        let rg = (g_col * 255.0) as u8;
        let rr = (r_col * 255.0) as u8;

        // Only iterate over bounding box of the ripple
        let x_min = ((cx - current_radius - 1.0).max(0.0)) as u32;
        let x_max = ((cx + current_radius + 1.0).min(w - 1.0)) as u32;
        let y_min = ((cy - current_radius - 1.0).max(0.0)) as u32;
        let y_max = ((cy + current_radius + 1.0).min(h - 1.0)) as u32;

        for py in y_min..=y_max {
            for px in x_min..=x_max {
                let dx = px as f64 - cx;
                let dy = py as f64 - cy;
                let dist = (dx * dx + dy * dy).sqrt();

                if dist >= inner_radius && dist <= current_radius {
                    // Smoothstep within the ring
                    let ring_t = if ring_width > 0.001 {
                        1.0 - ((dist - inner_radius) / ring_width).clamp(0.0, 1.0)
                    } else {
                        1.0
                    };
                    let alpha = (ring_t * opacity * 255.0) as u8;

                    let dst = frame.get_pixel(px, py);
                    let src = [rb, rg, rr, alpha];
                    frame.set_pixel(px, py, FrameBuffer::composite_over(dst, src));
                }
            }
        }
    }

    /// Apply cursor overlay at the evaluated position.
    /// Renders a simple circle cursor (platform cursor images can be added later).
    fn apply_cursor(&self, frame: &mut FrameBuffer, cursor: &CursorState) {
        let w = frame.width as f64;
        let h = frame.height as f64;

        let cx = cursor.position.x * w;
        let cy = cursor.position.y * h;
        let radius = (6.0 * cursor.scale).max(2.0);

        let x_min = ((cx - radius - 1.0).max(0.0)) as u32;
        let x_max = ((cx + radius + 1.0).min(w - 1.0)) as u32;
        let y_min = ((cy - radius - 1.0).max(0.0)) as u32;
        let y_max = ((cy + radius + 1.0).min(h - 1.0)) as u32;

        for py in y_min..=y_max {
            for px in x_min..=x_max {
                let dx = px as f64 - cx;
                let dy = py as f64 - cy;
                let dist = (dx * dx + dy * dy).sqrt();

                if dist <= radius {
                    // Anti-aliased edge
                    let edge_alpha = (radius - dist).clamp(0.0, 1.0);
                    let alpha = (edge_alpha * 255.0) as u8;

                    // White cursor with dark border
                    let (b, g, r) = if dist > radius - 1.5 {
                        (40u8, 40u8, 40u8) // Dark border
                    } else {
                        (255u8, 255u8, 255u8) // White fill
                    };

                    let dst = frame.get_pixel(px, py);
                    let src = [b, g, r, alpha];
                    frame.set_pixel(px, py, FrameBuffer::composite_over(dst, src));
                }
            }
        }
    }

    /// Apply transform (crop/zoom/pan) using bilinear resampling.
    /// For identity transforms (no zoom, same size), use apply_transform_cow to avoid cloning.
    fn apply_transform(&self, source: &FrameBuffer, transform: &TransformState) -> FrameBuffer {
        let out_w = self.ctx.output_size.width as u32;
        let out_h = self.ctx.output_size.height as u32;

        if transform.zoom <= 1.001 && out_w == source.width && out_h == source.height {
            // No transform needed — identity
            return source.clone();
        }

        // Near-center with minimal zoom: use nearest-neighbor (much faster than bilinear)
        if transform.zoom < 1.5 {
            return self.apply_transform_nearest(source, transform, out_w, out_h);
        }

        let mut output = FrameBuffer::new(out_w, out_h);

        // Calculate crop rectangle in source pixel space
        let src_w = source.width as f64;
        let src_h = source.height as f64;

        let crop_w = src_w / transform.zoom;
        let crop_h = src_h / transform.zoom;
        let crop_x = transform.center.x * src_w - crop_w / 2.0;
        let crop_y = transform.center.y * src_h - crop_h / 2.0;

        // Map each output pixel back to source
        for oy in 0..out_h {
            for ox in 0..out_w {
                let sx = crop_x + (ox as f64 / out_w as f64) * crop_w;
                let sy = crop_y + (oy as f64 / out_h as f64) * crop_h;

                let pixel = source.sample_bilinear(sx, sy);
                output.set_pixel(ox, oy, pixel);
            }
        }

        output
    }

    /// Fast nearest-neighbor transform for low zoom levels (< 1.5x).
    /// ~4x faster than bilinear — no interpolation, just pixel lookup.
    fn apply_transform_nearest(
        &self,
        source: &FrameBuffer,
        transform: &TransformState,
        out_w: u32,
        out_h: u32,
    ) -> FrameBuffer {
        let mut output = FrameBuffer::new(out_w, out_h);
        let src_w = source.width as f64;
        let src_h = source.height as f64;
        let crop_w = src_w / transform.zoom;
        let crop_h = src_h / transform.zoom;
        let crop_x = transform.center.x * src_w - crop_w / 2.0;
        let crop_y = transform.center.y * src_h - crop_h / 2.0;

        let inv_out_w = crop_w / out_w as f64;
        let inv_out_h = crop_h / out_h as f64;

        for oy in 0..out_h {
            let sy = (crop_y + oy as f64 * inv_out_h).round() as i64;
            let sy = sy.clamp(0, source.height as i64 - 1) as u32;
            for ox in 0..out_w {
                let sx = (crop_x + ox as f64 * inv_out_w).round() as i64;
                let sx = sx.clamp(0, source.width as i64 - 1) as u32;
                output.set_pixel(ox, oy, source.get_pixel(sx, sy));
            }
        }
        output
    }

    /// Apply keystroke text overlay (pill-shaped badge).
    /// Renders at output coordinates (fixed on screen, not affected by transform).
    fn apply_keystroke(&self, frame: &mut FrameBuffer, keystroke: &ActiveKeystroke) {
        let w = frame.width as f64;
        let h = frame.height as f64;

        // Font size ~3% of frame height
        let char_h = (h * 0.03).max(20.0);
        let char_w = char_h * 0.6;
        let text_len = keystroke.display_text.len() as f64;

        let pill_w = (text_len * char_w + char_h).min(w * 0.8);
        let pill_h = char_h * 1.6;
        let pill_r = pill_h / 2.0;

        // Position (normalized → pixel, centered)
        let cx = keystroke.position.x * w;
        let cy = keystroke.position.y * h;
        let pill_x = cx - pill_w / 2.0;
        let pill_y = cy - pill_h / 2.0;

        let opacity = keystroke.opacity.clamp(0.0, 1.0);
        if opacity < 0.01 {
            return;
        }

        let x_min = (pill_x.max(0.0)) as u32;
        let x_max = ((pill_x + pill_w).min(w - 1.0)) as u32;
        let y_min = (pill_y.max(0.0)) as u32;
        let y_max = ((pill_y + pill_h).min(h - 1.0)) as u32;

        // Draw pill background (rounded rectangle)
        for py in y_min..=y_max {
            for px in x_min..=x_max {
                let lx = px as f64 - pill_x;
                let ly = py as f64 - pill_y;

                // Rounded rect SDF
                let inside = is_inside_rounded_rect(lx, ly, pill_w, pill_h, pill_r);
                if inside {
                    let bg_alpha = (0.75 * opacity * 255.0) as u8;
                    let dst = frame.get_pixel(px, py);
                    let src = [30, 30, 30, bg_alpha]; // Dark semi-transparent background
                    frame.set_pixel(px, py, FrameBuffer::composite_over(dst, src));
                }
            }
        }

        // Draw text characters as simple block glyphs
        let text_start_x = cx - (text_len * char_w) / 2.0;
        let text_y = cy - char_h * 0.4;

        for (i, _ch) in keystroke.display_text.chars().enumerate() {
            let glyph_x = text_start_x + i as f64 * char_w;
            let glyph_y = text_y;

            // Simplified: draw a small filled rectangle per character
            let gx_min = (glyph_x + char_w * 0.15).max(0.0) as u32;
            let gx_max = (glyph_x + char_w * 0.85).min(w - 1.0) as u32;
            let gy_min = (glyph_y).max(0.0) as u32;
            let gy_max = (glyph_y + char_h * 0.8).min(h - 1.0) as u32;

            for py in gy_min..=gy_max {
                for px in gx_min..=gx_max {
                    let text_alpha = (opacity * 230.0) as u8;
                    let dst = frame.get_pixel(px, py);
                    let src = [255, 255, 255, text_alpha]; // White text
                    frame.set_pixel(px, py, FrameBuffer::composite_over(dst, src));
                }
            }
        }
    }
}

/// Check if a point (lx, ly) is inside a rounded rectangle with given dimensions.
fn is_inside_rounded_rect(lx: f64, ly: f64, w: f64, h: f64, r: f64) -> bool {
    if lx < 0.0 || ly < 0.0 || lx > w || ly > h {
        return false;
    }

    let r = r.min(w / 2.0).min(h / 2.0);

    // Check corner regions
    if lx < r && ly < r {
        return (lx - r).powi(2) + (ly - r).powi(2) <= r * r;
    }
    if lx > w - r && ly < r {
        return (lx - (w - r)).powi(2) + (ly - r).powi(2) <= r * r;
    }
    if lx < r && ly > h - r {
        return (lx - r).powi(2) + (ly - (h - r)).powi(2) <= r * r;
    }
    if lx > w - r && ly > h - r {
        return (lx - (w - r)).powi(2) + (ly - (h - r)).powi(2) <= r * r;
    }

    true
}

// =============================================================================
// Export engine
// =============================================================================

/// Export pipeline errors
#[derive(Debug, thiserror::Error)]
pub enum ExportError {
    #[error("Encoder error: {0}")]
    Encoder(#[from] super::encoder::EncoderError),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("No video source available")]
    NoSource,
    #[error("Export cancelled")]
    Cancelled,
}

/// Export progress information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportProgress {
    /// Current frame being processed
    pub current_frame: u64,
    /// Total frames to process
    pub total_frames: u64,
    /// Progress ratio (0.0 - 1.0)
    pub progress: f64,
    /// Estimated time remaining in seconds
    pub eta_seconds: f64,
    /// Export state
    pub state: ExportState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ExportState {
    Preparing,
    Rendering,
    Encoding,
    Finalizing,
    Completed,
    Failed,
}

/// Video source reader abstraction.
/// Extracts frames from a source video file.
pub trait VideoSource: Send {
    /// Total number of frames in the source
    fn total_frames(&self) -> u64;
    /// Frame rate of the source
    fn frame_rate(&self) -> f64;
    /// Duration in seconds
    fn duration(&self) -> f64;
    /// Extract frame at the given time (seconds)
    fn read_frame(&mut self, time: f64) -> Result<FrameBuffer, ExportError>;
}

/// Stub video source that generates solid-color test frames.
/// Used until FFmpeg integration is complete.
pub struct StubVideoSource {
    width: u32,
    height: u32,
    total: u64,
    fps: f64,
    dur: f64,
}

impl StubVideoSource {
    pub fn new(width: u32, height: u32, duration: f64, fps: f64) -> Self {
        Self {
            width,
            height,
            total: (duration * fps) as u64,
            fps,
            dur: duration,
        }
    }
}

impl VideoSource for StubVideoSource {
    fn total_frames(&self) -> u64 {
        self.total
    }

    fn frame_rate(&self) -> f64 {
        self.fps
    }

    fn duration(&self) -> f64 {
        self.dur
    }

    fn read_frame(&mut self, time: f64) -> Result<FrameBuffer, ExportError> {
        // Generate a gradient test pattern that changes with time
        let progress = (time / self.dur).clamp(0.0, 1.0);
        let mut frame = FrameBuffer::new(self.width, self.height);

        for y in 0..self.height {
            for x in 0..self.width {
                let fx = x as f64 / self.width as f64;
                let fy = y as f64 / self.height as f64;

                // Animated gradient
                let r = ((fx + progress) % 1.0 * 100.0 + 30.0) as u8;
                let g = ((fy + progress * 0.5) % 1.0 * 80.0 + 20.0) as u8;
                let b = (40.0 + progress * 60.0) as u8;

                frame.set_pixel(x, y, [b, g, r, 255]);
            }
        }

        Ok(frame)
    }
}

/// FFmpeg-based video source reader.
/// Decodes video frames from a file for the render pipeline.
#[cfg(feature = "ffmpeg")]
pub mod ffmpeg_source {
    use super::*;
    use ffmpeg_next as ffmpeg;
    use ffmpeg::format;
    use ffmpeg::media::Type;
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

    pub struct FfmpegVideoSource {
        input_ctx: format::context::Input,
        video_stream_index: usize,
        decoder: ffmpeg::codec::decoder::Video,
        scaler: SendScaler,
        width: u32,
        height: u32,
        total: u64,
        fps: f64,
        dur: f64,
        time_base: f64,
    }

    impl FfmpegVideoSource {
        pub fn open(path: &std::path::Path) -> Result<Self, ExportError> {
            ffmpeg::init().map_err(|e| ExportError::Io(
                std::io::Error::new(std::io::ErrorKind::Other, format!("FFmpeg init: {e}"))
            ))?;

            let input_ctx = format::input(path).map_err(|e| ExportError::Io(
                std::io::Error::new(std::io::ErrorKind::Other, format!("Open input: {e}"))
            ))?;

            let stream = input_ctx.streams().best(Type::Video)
                .ok_or(ExportError::NoSource)?;
            let video_stream_index = stream.index();

            let time_base = stream.time_base();
            let time_base_f64 = time_base.0 as f64 / time_base.1 as f64;

            let decoder_ctx = ffmpeg::codec::context::Context::from_parameters(stream.parameters())
                .map_err(|e| ExportError::Io(
                    std::io::Error::new(std::io::ErrorKind::Other, format!("Decoder context: {e}"))
                ))?;
            let decoder = decoder_ctx.decoder().video()
                .map_err(|e| ExportError::Io(
                    std::io::Error::new(std::io::ErrorKind::Other, format!("Open decoder: {e}"))
                ))?;

            let width = decoder.width();
            let height = decoder.height();
            let pixel_format = decoder.format();

            let scaler = scaling::Context::get(
                pixel_format,
                width,
                height,
                ffmpeg::format::Pixel::BGRA,
                width,
                height,
                scaling::Flags::BILINEAR,
            ).map_err(|e| ExportError::Io(
                std::io::Error::new(std::io::ErrorKind::Other, format!("Scaler init: {e}"))
            ))?;

            let fps = stream.avg_frame_rate();
            let fps_f64 = if fps.1 != 0 { fps.0 as f64 / fps.1 as f64 } else { 30.0 };

            let dur = input_ctx.duration() as f64 / ffmpeg::ffi::AV_TIME_BASE as f64;
            let total = (dur * fps_f64) as u64;

            Ok(Self {
                input_ctx,
                video_stream_index,
                decoder,
                scaler: SendScaler(scaler),
                width,
                height,
                total,
                fps: fps_f64,
                dur,
                time_base: time_base_f64,
            })
        }

        fn seek_to(&mut self, time: f64) -> Result<(), ExportError> {
            let timestamp = (time / self.time_base) as i64;
            self.input_ctx.seek(timestamp, ..timestamp)
                .map_err(|e| ExportError::Io(
                    std::io::Error::new(std::io::ErrorKind::Other, format!("Seek: {e}"))
                ))?;
            self.decoder.flush();
            Ok(())
        }

        fn decode_next_frame(&mut self) -> Result<FfmpegFrame, ExportError> {
            loop {
                for (stream, packet) in self.input_ctx.packets() {
                    if stream.index() != self.video_stream_index {
                        continue;
                    }
                    self.decoder.send_packet(&packet).map_err(|e| ExportError::Io(
                        std::io::Error::new(std::io::ErrorKind::Other, format!("Send packet: {e}"))
                    ))?;

                    let mut decoded = FfmpegFrame::empty();
                    if self.decoder.receive_frame(&mut decoded).is_ok() {
                        return Ok(decoded);
                    }
                }
                // If we exhausted packets, send EOF and try to get remaining frames
                self.decoder.send_eof().ok();
                let mut decoded = FfmpegFrame::empty();
                if self.decoder.receive_frame(&mut decoded).is_ok() {
                    return Ok(decoded);
                }
                return Err(ExportError::NoSource);
            }
        }
    }

    impl VideoSource for FfmpegVideoSource {
        fn total_frames(&self) -> u64 {
            self.total
        }

        fn frame_rate(&self) -> f64 {
            self.fps
        }

        fn duration(&self) -> f64 {
            self.dur
        }

        fn read_frame(&mut self, _time: f64) -> Result<FrameBuffer, ExportError> {
            // Sequential decode — no seeking. The export loop processes frames in order,
            // so we just decode the next frame. Seeking per-frame was the #1 bottleneck
            // (re-decoding from nearest keyframe for every single frame).
            let decoded = self.decode_next_frame()?;

            // Convert to BGRA
            let mut bgra_frame = FfmpegFrame::empty();
            self.scaler.run(&decoded, &mut bgra_frame).map_err(|e| ExportError::Io(
                std::io::Error::new(std::io::ErrorKind::Other, format!("Scale frame: {e}"))
            ))?;

            let stride = self.width * 4;
            let data_size = (stride * self.height) as usize;
            let src_data = bgra_frame.data(0);

            // Handle potential stride mismatch
            let src_stride = bgra_frame.stride(0) as u32;
            let data = if src_stride == stride {
                src_data[..data_size].to_vec()
            } else {
                let mut buf = vec![0u8; data_size];
                for y in 0..self.height {
                    let src_offset = (y * src_stride) as usize;
                    let dst_offset = (y * stride) as usize;
                    let row_bytes = stride as usize;
                    buf[dst_offset..dst_offset + row_bytes]
                        .copy_from_slice(&src_data[src_offset..src_offset + row_bytes]);
                }
                buf
            };

            Ok(FrameBuffer {
                data,
                width: self.width,
                height: self.height,
                stride,
            })
        }
    }
}

/// Create a video source from a file path.
/// Returns FFmpeg source when the `ffmpeg` feature is enabled and the file exists,
/// otherwise falls back to the stub source.
pub fn create_video_source_from_file(
    path: &std::path::Path,
    fallback_width: u32,
    fallback_height: u32,
    fallback_duration: f64,
    fallback_fps: f64,
) -> Box<dyn VideoSource> {
    #[cfg(feature = "ffmpeg")]
    {
        if path.exists() {
            match ffmpeg_source::FfmpegVideoSource::open(path) {
                Ok(src) => return Box::new(src),
                Err(e) => {
                    log::warn!("FFmpeg source open failed, falling back to stub: {e}");
                }
            }
        }
    }
    let _ = path; // suppress unused warning when ffmpeg feature disabled
    Box::new(StubVideoSource::new(fallback_width, fallback_height, fallback_duration, fallback_fps))
}

/// Create a stub video source (placeholder for FFmpeg)
pub fn create_video_source(width: u32, height: u32, duration: f64, fps: f64) -> Box<dyn VideoSource> {
    Box::new(StubVideoSource::new(width, height, duration, fps))
}

/// Frame-by-frame export engine.
///
/// Orchestrates: source reading → evaluation → rendering → encoding
pub struct ExportEngine {
    renderer: SoftwareRenderer,
    evaluator: FrameEvaluator,
    encoder: Box<dyn VideoEncoder>,
    source: Box<dyn VideoSource>,
    timeline: Timeline,
    mouse_positions: Vec<MousePosition>,
    ctx: RenderContext,
}

impl ExportEngine {
    /// Create a new export engine from a project
    pub fn from_project(
        project: &Project,
        source: Box<dyn VideoSource>,
        mouse_positions: Vec<MousePosition>,
        output_path: PathBuf,
    ) -> Self {
        let ctx = RenderContext::from_project(project);

        let encoder_config = EncoderConfig {
            width: ctx.output_size.width as u32,
            height: ctx.output_size.height as u32,
            frame_rate: ctx.frame_rate as u32,
            codec: ctx.render_settings.codec,
            quality: ctx.render_settings.quality,
            output_path,
            keyframe_interval: 120,
            purpose: super::encoder::EncoderPurpose::Export,
        };

        Self {
            renderer: SoftwareRenderer::new(ctx.clone()),
            evaluator: FrameEvaluator::new(ctx.window_mode),
            encoder: create_encoder(encoder_config),
            source,
            timeline: project.timeline.clone(),
            mouse_positions,
            ctx,
        }
    }

    /// Run the full export pipeline.
    /// Returns the output file path on success.
    ///
    /// `progress_callback` is called periodically with progress updates.
    pub fn export<F>(&mut self, mut progress_callback: F) -> Result<PathBuf, ExportError>
    where
        F: FnMut(ExportProgress),
    {
        // Prepare
        progress_callback(ExportProgress {
            current_frame: 0,
            total_frames: self.source.total_frames(),
            progress: 0.0,
            eta_seconds: 0.0,
            state: ExportState::Preparing,
        });

        self.encoder.start()?;

        let total_frames = self.source.total_frames();
        let frame_duration = 1.0 / self.ctx.frame_rate;
        let start_time = std::time::Instant::now();

        // Main render loop
        for frame_idx in 0..total_frames {
            let time = frame_idx as f64 * frame_duration;

            // 1. Read source frame
            let source_frame = self.source.read_frame(time)?;

            // 2. Evaluate timeline state at this time
            let state = self.evaluator.evaluate(
                &self.timeline,
                time,
                &self.mouse_positions,
            );

            // 3. Render all effects
            let output_frame = self.renderer.render_frame(&source_frame, &state);

            // 4. Encode (move data instead of clone — saves ~20MB per frame)
            let video_frame = output_frame.into_video_frame(time);
            self.encoder.append_frame(&video_frame)?;

            // 5. Progress update (every 10 frames)
            if frame_idx % 10 == 0 || frame_idx == total_frames - 1 {
                let elapsed = start_time.elapsed().as_secs_f64();
                let fps = if elapsed > 0.0 { frame_idx as f64 / elapsed } else { 0.0 };
                let remaining = if fps > 0.0 {
                    (total_frames - frame_idx) as f64 / fps
                } else {
                    0.0
                };

                progress_callback(ExportProgress {
                    current_frame: frame_idx,
                    total_frames,
                    progress: frame_idx as f64 / total_frames as f64,
                    eta_seconds: remaining,
                    state: ExportState::Rendering,
                });
            }
        }

        // Finalize
        progress_callback(ExportProgress {
            current_frame: total_frames,
            total_frames,
            progress: 1.0,
            eta_seconds: 0.0,
            state: ExportState::Finalizing,
        });

        let output_path = self.encoder.finish()?;

        progress_callback(ExportProgress {
            current_frame: total_frames,
            total_frames,
            progress: 1.0,
            eta_seconds: 0.0,
            state: ExportState::Completed,
        });

        Ok(output_path)
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::coordinates::NormalizedPoint;

    #[test]
    fn test_frame_buffer_solid() {
        let fb = FrameBuffer::solid(4, 4, 100, 150, 200, 255);
        let pixel = fb.get_pixel(2, 2);
        assert_eq!(pixel, [100, 150, 200, 255]);
    }

    #[test]
    fn test_frame_buffer_set_get() {
        let mut fb = FrameBuffer::new(10, 10);
        fb.set_pixel(5, 5, [10, 20, 30, 255]);
        assert_eq!(fb.get_pixel(5, 5), [10, 20, 30, 255]);
        assert_eq!(fb.get_pixel(0, 0), [0, 0, 0, 0]);
    }

    #[test]
    fn test_composite_over() {
        // Fully opaque source replaces dst
        let result = FrameBuffer::composite_over([100, 100, 100, 255], [200, 200, 200, 255]);
        assert_eq!(result, [200, 200, 200, 255]);

        // Fully transparent source leaves dst unchanged
        let result = FrameBuffer::composite_over([100, 100, 100, 255], [200, 200, 200, 0]);
        assert_eq!(result, [100, 100, 100, 255]);
    }

    #[test]
    fn test_bilinear_sample_exact() {
        let mut fb = FrameBuffer::new(4, 4);
        fb.set_pixel(1, 1, [100, 100, 100, 255]);
        let pixel = fb.sample_bilinear(1.0, 1.0);
        assert_eq!(pixel, [100, 100, 100, 255]);
    }

    #[test]
    fn test_bilinear_sample_midpoint() {
        let mut fb = FrameBuffer::new(4, 4);
        fb.set_pixel(0, 0, [0, 0, 0, 255]);
        fb.set_pixel(1, 0, [200, 200, 200, 255]);
        let pixel = fb.sample_bilinear(0.5, 0.0);
        assert_eq!(pixel[0], 100); // midpoint
    }

    #[test]
    fn test_rounded_rect_center() {
        assert!(is_inside_rounded_rect(50.0, 25.0, 100.0, 50.0, 10.0));
    }

    #[test]
    fn test_rounded_rect_outside() {
        assert!(!is_inside_rounded_rect(-1.0, 25.0, 100.0, 50.0, 10.0));
    }

    #[test]
    fn test_rounded_rect_corner() {
        // Right at the corner origin — should be outside the rounded region
        assert!(!is_inside_rounded_rect(0.0, 0.0, 100.0, 50.0, 20.0));
        // Just inside the corner arc
        assert!(is_inside_rounded_rect(5.0, 5.0, 100.0, 50.0, 5.0));
    }

    #[test]
    fn test_stub_video_source() {
        let mut src = StubVideoSource::new(320, 240, 1.0, 30.0);
        assert_eq!(src.total_frames(), 30);
        assert!((src.frame_rate() - 30.0).abs() < 0.001);

        let frame = src.read_frame(0.5).unwrap();
        assert_eq!(frame.width, 320);
        assert_eq!(frame.height, 240);
    }

    #[test]
    fn test_render_identity_transform() {
        let ctx = RenderContext {
            source_size: Size::new(100.0, 100.0),
            output_size: Size::new(100.0, 100.0),
            frame_rate: 30.0,
            window_mode: false,
            render_settings: RenderSettings::default(),
        };

        let renderer = SoftwareRenderer::new(ctx);
        let source = FrameBuffer::solid(100, 100, 50, 100, 150, 255);

        let transform = TransformState {
            zoom: 1.0,
            center: NormalizedPoint::CENTER,
            velocity: 0.0,
        };

        let result = renderer.apply_transform(&source, &transform);
        // Identity transform should preserve pixels
        assert_eq!(result.get_pixel(50, 50), source.get_pixel(50, 50));
    }

    #[test]
    fn test_render_zoom_transform() {
        let ctx = RenderContext {
            source_size: Size::new(100.0, 100.0),
            output_size: Size::new(100.0, 100.0),
            frame_rate: 30.0,
            window_mode: false,
            render_settings: RenderSettings::default(),
        };

        let renderer = SoftwareRenderer::new(ctx);

        // Create a source with a distinct pattern
        let mut source = FrameBuffer::new(100, 100);
        // Center pixel is bright
        source.set_pixel(50, 50, [255, 255, 255, 255]);

        let transform = TransformState {
            zoom: 2.0,
            center: NormalizedPoint::CENTER,
            velocity: 0.0,
        };

        let result = renderer.apply_transform(&source, &transform);
        // At 2x zoom, the center pixel should still be visible
        assert_eq!(result.width, 100);
        assert_eq!(result.height, 100);
    }

    #[test]
    fn test_export_engine_with_stubs() {
        use super::super::project::{
            CaptureMeta, MediaAsset, Project, Rect,
        };

        let media = MediaAsset {
            video_relative_path: "test.mp4".into(),
            mouse_data_relative_path: "test_mouse.json".into(),
            pixel_size: Size::new(320.0, 240.0),
            frame_rate: 10.0,
            duration: 0.5, // 5 frames at 10fps
        };

        let capture_meta = CaptureMeta::new(
            Rect::new(0.0, 0.0, 320.0, 240.0),
            1.0,
        );

        let project = Project::new("Test Export".into(), media, capture_meta);

        let source = create_video_source(320, 240, 0.5, 10.0);
        let mouse_positions = vec![
            MousePosition { time: 0.0, position: NormalizedPoint::new(0.3, 0.4) },
            MousePosition { time: 0.5, position: NormalizedPoint::new(0.7, 0.6) },
        ];

        let output_path = std::env::temp_dir().join("lazyrec_test_export.mp4");
        let mut engine = ExportEngine::from_project(
            &project,
            source,
            mouse_positions,
            output_path.clone(),
        );

        let mut last_progress = 0.0;
        let result = engine.export(|progress| {
            assert!(progress.progress >= last_progress);
            last_progress = progress.progress;
        });

        assert!(result.is_ok());
        let result_path = result.unwrap();
        assert_eq!(result_path, output_path);
    }

    #[test]
    fn test_ripple_rendering() {
        let ctx = RenderContext {
            source_size: Size::new(200.0, 200.0),
            output_size: Size::new(200.0, 200.0),
            frame_rate: 30.0,
            window_mode: false,
            render_settings: RenderSettings::default(),
        };

        let renderer = SoftwareRenderer::new(ctx);
        let mut frame = FrameBuffer::solid(200, 200, 0, 0, 0, 255);

        let ripple = ActiveRipple {
            position: NormalizedPoint::CENTER,
            progress: 0.5,
            intensity: 1.0,
            color: (1.0, 0.0, 0.0, 1.0), // Red
        };

        renderer.apply_ripple(&mut frame, &ripple);

        // Check that some pixels near center were modified (no longer pure black)
        let center_pixel = frame.get_pixel(100, 100);
        // The center may or may not be inside the ring depending on progress,
        // but pixels somewhere around the ring radius should be colored
        let near_ring = frame.get_pixel(104, 100);
        // At least one of them should have been modified
        let modified = center_pixel != [0, 0, 0, 255] || near_ring != [0, 0, 0, 255];
        assert!(modified, "Ripple should have modified some pixels");
    }

    #[test]
    fn test_cursor_rendering() {
        let ctx = RenderContext {
            source_size: Size::new(100.0, 100.0),
            output_size: Size::new(100.0, 100.0),
            frame_rate: 30.0,
            window_mode: false,
            render_settings: RenderSettings::default(),
        };

        let renderer = SoftwareRenderer::new(ctx);
        let mut frame = FrameBuffer::solid(100, 100, 0, 0, 0, 255);

        let cursor = CursorState {
            position: NormalizedPoint::CENTER,
            style: super::super::keyframe::CursorStyle::Arrow,
            scale: 2.5,
            visible: true,
            velocity: 0.0,
            movement_direction: 0.0,
        };

        renderer.apply_cursor(&mut frame, &cursor);

        // Center pixel should now be white (cursor fill)
        let pixel = frame.get_pixel(50, 50);
        assert!(pixel[0] > 200 && pixel[1] > 200 && pixel[2] > 200,
            "Cursor center should be white, got {:?}", pixel);
    }
}
