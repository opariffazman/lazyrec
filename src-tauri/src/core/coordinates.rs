use serde::{Deserialize, Serialize};

/// Normalized coordinates (0-1, top-left origin) used as the internal standard.
/// Unlike Screenize's bottom-left origin, we use top-left to match Windows/Linux conventions.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct NormalizedPoint {
    /// 0.0-1.0, left is 0
    pub x: f64,
    /// 0.0-1.0, top is 0
    pub y: f64,
}

impl NormalizedPoint {
    pub const ZERO: Self = Self { x: 0.0, y: 0.0 };
    pub const CENTER: Self = Self { x: 0.5, y: 0.5 };

    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    /// Returns clamped coordinates (ensures 0-1 range)
    pub fn clamped(&self) -> Self {
        Self {
            x: self.x.clamp(0.0, 1.0),
            y: self.y.clamp(0.0, 1.0),
        }
    }

    /// Euclidean distance between two points
    pub fn distance(&self, other: &Self) -> f64 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        (dx * dx + dy * dy).sqrt()
    }

    /// Interpolate toward another point
    pub fn interpolated(&self, to: &Self, amount: f64) -> Self {
        let t = amount.clamp(0.0, 1.0);
        Self {
            x: self.x + (to.x - self.x) * t,
            y: self.y + (to.y - self.y) * t,
        }
    }
}

impl std::hash::Hash for NormalizedPoint {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.x.to_bits().hash(state);
        self.y.to_bits().hash(state);
    }
}

impl Eq for NormalizedPoint {}

// Collection operations for NormalizedPoint slices

/// Calculate the bounding box of a set of points
pub fn bounding_box(points: &[NormalizedPoint]) -> Option<(f64, f64, f64, f64)> {
    if points.is_empty() {
        return None;
    }
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;

    for p in points {
        min_x = min_x.min(p.x);
        min_y = min_y.min(p.y);
        max_x = max_x.max(p.x);
        max_y = max_y.max(p.y);
    }
    Some((min_x, min_y, max_x, max_y))
}

/// Calculate the centroid (average) of a set of points
pub fn centroid(points: &[NormalizedPoint]) -> Option<NormalizedPoint> {
    if points.is_empty() {
        return None;
    }
    let count = points.len() as f64;
    let sum_x: f64 = points.iter().map(|p| p.x).sum();
    let sum_y: f64 = points.iter().map(|p| p.y).sum();
    Some(NormalizedPoint::new(sum_x / count, sum_y / count))
}

/// Return the bounding box center
pub fn bounding_box_center(points: &[NormalizedPoint]) -> Option<NormalizedPoint> {
    bounding_box(points).map(|(min_x, min_y, max_x, max_y)| {
        NormalizedPoint::new((min_x + max_x) / 2.0, (min_y + max_y) / 2.0)
    })
}

// Viewport operations

/// Compute viewport bounds based on zoom level and center
pub fn viewport_bounds(zoom: f64, center: &NormalizedPoint) -> (f64, f64, f64, f64) {
    let half_width = 0.5 / zoom.max(1.0);
    let half_height = 0.5 / zoom.max(1.0);
    (
        center.x - half_width,
        center.x + half_width,
        center.y - half_height,
        center.y + half_height,
    )
}

impl NormalizedPoint {
    /// Determine if this point lies outside the viewport
    pub fn is_outside_viewport(&self, zoom: f64, center: &Self, margin: f64) -> bool {
        if zoom <= 1.0 {
            return false;
        }
        let (min_x, max_x, min_y, max_y) = viewport_bounds(zoom, center);
        let effective_margin = margin / zoom;
        self.x < (min_x + effective_margin)
            || self.x > (max_x - effective_margin)
            || self.y < (min_y + effective_margin)
            || self.y > (max_y - effective_margin)
    }

    /// Calculate a new center to keep this point within the viewport
    pub fn center_to_include_in_viewport(
        &self,
        zoom: f64,
        current_center: &Self,
        padding: f64,
    ) -> Self {
        if zoom <= 1.0 {
            return *current_center;
        }

        let half_width = 0.5 / zoom;
        let half_height = 0.5 / zoom;
        let effective_padding = padding / zoom;

        let mut new_x = current_center.x;
        let mut new_y = current_center.y;

        let left_bound = current_center.x - half_width + effective_padding;
        let right_bound = current_center.x + half_width - effective_padding;

        if self.x < left_bound {
            new_x = self.x + half_width - effective_padding;
        } else if self.x > right_bound {
            new_x = self.x - half_width + effective_padding;
        }

        let top_bound = current_center.y - half_height + effective_padding;
        let bottom_bound = current_center.y + half_height - effective_padding;

        if self.y < top_bound {
            new_y = self.y + half_height - effective_padding;
        } else if self.y > bottom_bound {
            new_y = self.y - half_height + effective_padding;
        }

        Self {
            x: new_x.clamp(half_width, 1.0 - half_width),
            y: new_y.clamp(half_height, 1.0 - half_height),
        }
    }

    /// Relative position of the point within the viewport (0-1, 0.5 is centered)
    pub fn relative_position_in_viewport(&self, zoom: f64, center: &Self) -> Self {
        if zoom <= 1.0 {
            return *self;
        }
        let (min_x, max_x, min_y, max_y) = viewport_bounds(zoom, center);
        let vw = max_x - min_x;
        let vh = max_y - min_y;
        Self {
            x: (self.x - min_x) / vw,
            y: (self.y - min_y) / vh,
        }
    }
}

/// Pixel coordinates relative to the capture area.
/// Used when saving mouse recording data.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CapturePixelPoint {
    pub x: f64,
    pub y: f64,
}

impl CapturePixelPoint {
    pub const ZERO: Self = Self { x: 0.0, y: 0.0 };

    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

/// Platform screen coordinates (origin depends on platform).
/// On Windows/Linux: top-left origin. On macOS: bottom-left origin.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScreenPoint {
    pub x: f64,
    pub y: f64,
}

impl ScreenPoint {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

/// Coordinate conversion utilities.
/// Centralizes conversions to ensure consistency.
pub struct CoordinateConverter {
    /// Capture bounds in platform points
    pub capture_bounds_x: f64,
    pub capture_bounds_y: f64,
    pub capture_bounds_width: f64,
    pub capture_bounds_height: f64,
    /// Scale factor (HiDPI: 2.0, standard: 1.0)
    pub scale_factor: f64,
}

impl CoordinateConverter {
    pub fn new(
        capture_x: f64,
        capture_y: f64,
        capture_width: f64,
        capture_height: f64,
        scale_factor: f64,
    ) -> Self {
        Self {
            capture_bounds_x: capture_x,
            capture_bounds_y: capture_y,
            capture_bounds_width: capture_width,
            capture_bounds_height: capture_height,
            scale_factor,
        }
    }

    /// Capture size in pixels
    pub fn capture_size_pixel(&self) -> (f64, f64) {
        (
            self.capture_bounds_width * self.scale_factor,
            self.capture_bounds_height * self.scale_factor,
        )
    }

    /// Convert screen coordinates to capture pixel coordinates
    pub fn screen_to_capture_pixel(&self, screen: &ScreenPoint) -> CapturePixelPoint {
        CapturePixelPoint {
            x: screen.x - self.capture_bounds_x,
            y: screen.y - self.capture_bounds_y,
        }
    }

    /// Convert capture pixel coordinates to normalized coordinates (0-1)
    pub fn capture_pixel_to_normalized(&self, pixel: &CapturePixelPoint) -> NormalizedPoint {
        if self.capture_bounds_width <= 0.0 || self.capture_bounds_height <= 0.0 {
            return NormalizedPoint::CENTER;
        }
        NormalizedPoint {
            x: (pixel.x / self.capture_bounds_width).clamp(0.0, 1.0),
            y: (pixel.y / self.capture_bounds_height).clamp(0.0, 1.0),
        }
    }

    /// Convert screen coordinates to normalized coordinates in one step
    pub fn screen_to_normalized(&self, screen: &ScreenPoint) -> NormalizedPoint {
        let pixel = self.screen_to_capture_pixel(screen);
        self.capture_pixel_to_normalized(&pixel)
    }

    /// Convert normalized coordinates to video frame pixel coordinates
    pub fn normalized_to_video_pixel(
        normalized: &NormalizedPoint,
        video_width: f64,
        video_height: f64,
    ) -> (f64, f64) {
        (normalized.x * video_width, normalized.y * video_height)
    }

    /// Convert normalized coordinates to pixel coordinates
    pub fn normalized_to_pixel(
        normalized: &NormalizedPoint,
        width: f64,
        height: f64,
    ) -> (f64, f64) {
        (normalized.x * width, normalized.y * height)
    }

    /// Convert pixel coordinates to normalized coordinates
    pub fn pixel_to_normalized(px: f64, py: f64, width: f64, height: f64) -> NormalizedPoint {
        if width <= 0.0 || height <= 0.0 {
            return NormalizedPoint::CENTER;
        }
        NormalizedPoint {
            x: (px / width).clamp(0.0, 1.0),
            y: (py / height).clamp(0.0, 1.0),
        }
    }

    /// Convert normalized coordinates back to capture pixel coordinates
    pub fn normalized_to_capture_pixel(&self, normalized: &NormalizedPoint) -> CapturePixelPoint {
        CapturePixelPoint {
            x: normalized.x * self.capture_bounds_width,
            y: normalized.y * self.capture_bounds_height,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalized_point_clamp() {
        let p = NormalizedPoint::new(-0.1, 1.5);
        let c = p.clamped();
        assert_eq!(c.x, 0.0);
        assert_eq!(c.y, 1.0);
    }

    #[test]
    fn test_distance() {
        let a = NormalizedPoint::new(0.0, 0.0);
        let b = NormalizedPoint::new(1.0, 0.0);
        assert!((a.distance(&b) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_interpolation() {
        let a = NormalizedPoint::new(0.0, 0.0);
        let b = NormalizedPoint::new(1.0, 1.0);
        let mid = a.interpolated(&b, 0.5);
        assert!((mid.x - 0.5).abs() < 1e-10);
        assert!((mid.y - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_bounding_box() {
        let points = vec![
            NormalizedPoint::new(0.2, 0.3),
            NormalizedPoint::new(0.8, 0.1),
            NormalizedPoint::new(0.5, 0.9),
        ];
        let (min_x, min_y, max_x, max_y) = bounding_box(&points).unwrap();
        assert!((min_x - 0.2).abs() < 1e-10);
        assert!((min_y - 0.1).abs() < 1e-10);
        assert!((max_x - 0.8).abs() < 1e-10);
        assert!((max_y - 0.9).abs() < 1e-10);
    }

    #[test]
    fn test_coordinate_converter() {
        let conv = CoordinateConverter::new(100.0, 200.0, 800.0, 600.0, 2.0);
        let screen = ScreenPoint::new(500.0, 500.0);
        let pixel = conv.screen_to_capture_pixel(&screen);
        assert!((pixel.x - 400.0).abs() < 1e-10);
        assert!((pixel.y - 300.0).abs() < 1e-10);
        let norm = conv.capture_pixel_to_normalized(&pixel);
        assert!((norm.x - 0.5).abs() < 1e-10);
        assert!((norm.y - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_viewport_bounds() {
        let (min_x, max_x, min_y, max_y) = viewport_bounds(2.0, &NormalizedPoint::CENTER);
        assert!((min_x - 0.25).abs() < 1e-10);
        assert!((max_x - 0.75).abs() < 1e-10);
        assert!((min_y - 0.25).abs() < 1e-10);
        assert!((max_y - 0.75).abs() < 1e-10);
    }
}
