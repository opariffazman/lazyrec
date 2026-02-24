use serde::{Deserialize, Serialize};

/// Easing curve types.
/// Defines how interpolation happens between keyframes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum EasingCurve {
    #[serde(rename = "linear")]
    Linear,
    #[serde(rename = "easeIn")]
    EaseIn,
    #[serde(rename = "easeOut")]
    EaseOut,
    #[serde(rename = "easeInOut")]
    EaseInOut,
    #[serde(rename = "cubicBezier")]
    CubicBezier {
        p1x: f64,
        p1y: f64,
        p2x: f64,
        p2y: f64,
    },
    #[serde(rename = "spring")]
    Spring {
        #[serde(rename = "dampingRatio")]
        damping_ratio: f64,
        response: f64,
    },
}

impl EasingCurve {
    /// Apply the easing function.
    /// - `t`: Progress (0.0-1.0)
    /// - `duration`: Actual duration of the keyframe segment in seconds (used by spring)
    pub fn apply(&self, t: f64, duration: f64) -> f64 {
        let t = t.clamp(0.0, 1.0);
        let result = match self {
            Self::Linear => t,
            Self::EaseIn => t * t,
            Self::EaseOut => t * (2.0 - t),
            Self::EaseInOut => {
                if t < 0.5 {
                    2.0 * t * t
                } else {
                    -1.0 + (4.0 - 2.0 * t) * t
                }
            }
            Self::CubicBezier { p1x, p1y, p2x, p2y } => {
                cubic_bezier_value(t, *p1x, *p1y, *p2x, *p2y)
            }
            Self::Spring { .. } => self.spring_value(t, duration),
        };
        result.clamp(0.0, 1.0)
    }

    /// Return the raw value without clamping output
    pub fn apply_unclamped(&self, t: f64) -> f64 {
        match self {
            Self::Linear => t,
            Self::EaseIn => t * t,
            Self::EaseOut => t * (2.0 - t),
            Self::EaseInOut => {
                if t < 0.5 {
                    2.0 * t * t
                } else {
                    -1.0 + (4.0 - 2.0 * t) * t
                }
            }
            Self::CubicBezier { p1x, p1y, p2x, p2y } => {
                cubic_bezier_value(t, *p1x, *p1y, *p2x, *p2y)
            }
            Self::Spring { .. } => self.spring_value(t, 1.0),
        }
    }

    /// Compute the derivative (velocity) of the easing function.
    /// Used for calculating motion blur intensity.
    pub fn derivative(&self, t: f64, duration: f64) -> f64 {
        let t = t.clamp(0.0, 1.0);
        match self {
            Self::Linear => 1.0,
            Self::EaseIn => 2.0 * t,
            Self::EaseOut => 2.0 - 2.0 * t,
            Self::EaseInOut => {
                if t < 0.5 {
                    4.0 * t
                } else {
                    4.0 - 4.0 * t
                }
            }
            Self::CubicBezier { p1x, p1y, p2x, p2y } => {
                cubic_bezier_derivative(t, *p1x, *p1y, *p2x, *p2y)
            }
            Self::Spring { .. } => self.spring_derivative(t, duration),
        }
    }

    // Spring calculation: critically damped spring
    fn spring_value(&self, t: f64, duration: f64) -> f64 {
        let response = duration * 0.5;
        let omega = 2.0 * std::f64::consts::PI / response.max(0.01);
        let actual_time = t * duration;
        let decay = (-omega * actual_time).exp();
        1.0 - (1.0 + omega * actual_time) * decay
    }

    fn spring_derivative(&self, t: f64, duration: f64) -> f64 {
        let response = duration * 0.5;
        let omega = 2.0 * std::f64::consts::PI / response.max(0.01);
        let actual_time = t * duration;
        let decay = (-omega * actual_time).exp();
        omega * omega * actual_time * decay * duration
    }

    // Presets

    pub fn spring_default() -> Self {
        Self::Spring { damping_ratio: 1.0, response: 0.8 }
    }

    pub fn spring_smooth() -> Self {
        Self::Spring { damping_ratio: 1.0, response: 1.0 }
    }

    pub fn spring_bouncy() -> Self {
        Self::Spring { damping_ratio: 0.75, response: 0.9 }
    }

    pub fn spring_snappy() -> Self {
        Self::Spring { damping_ratio: 0.95, response: 0.5 }
    }

    pub fn css_ease() -> Self {
        Self::CubicBezier { p1x: 0.25, p1y: 0.1, p2x: 0.25, p2y: 1.0 }
    }

    pub fn css_ease_in() -> Self {
        Self::CubicBezier { p1x: 0.42, p1y: 0.0, p2x: 1.0, p2y: 1.0 }
    }

    pub fn css_ease_out() -> Self {
        Self::CubicBezier { p1x: 0.0, p1y: 0.0, p2x: 0.58, p2y: 1.0 }
    }

    pub fn css_ease_in_out() -> Self {
        Self::CubicBezier { p1x: 0.42, p1y: 0.0, p2x: 0.58, p2y: 1.0 }
    }

    pub fn display_name(&self) -> &str {
        match self {
            Self::Linear => "Linear",
            Self::EaseIn => "Ease In",
            Self::EaseOut => "Ease Out",
            Self::EaseInOut => "Ease In Out",
            Self::CubicBezier { .. } => "Custom Bezier",
            Self::Spring { damping_ratio, .. } => {
                if *damping_ratio >= 1.0 {
                    "Spring (Smooth)"
                } else if *damping_ratio >= 0.7 {
                    "Spring"
                } else {
                    "Spring (Bouncy)"
                }
            }
        }
    }

    pub fn is_spring(&self) -> bool {
        matches!(self, Self::Spring { .. })
    }
}

// Cubic bezier helpers

fn bezier_x(t: f64, p1x: f64, p2x: f64) -> f64 {
    let t2 = t * t;
    let t3 = t2 * t;
    let mt = 1.0 - t;
    let mt2 = mt * mt;
    3.0 * mt2 * t * p1x + 3.0 * mt * t2 * p2x + t3
}

fn bezier_y(t: f64, p1y: f64, p2y: f64) -> f64 {
    let t2 = t * t;
    let t3 = t2 * t;
    let mt = 1.0 - t;
    let mt2 = mt * mt;
    3.0 * mt2 * t * p1y + 3.0 * mt * t2 * p2y + t3
}

fn bezier_x_derivative(t: f64, p1x: f64, p2x: f64) -> f64 {
    let t2 = t * t;
    let mt = 1.0 - t;
    3.0 * mt * mt * p1x + 6.0 * mt * t * (p2x - p1x) + 3.0 * t2 * (1.0 - p2x)
}

fn bezier_y_derivative(t: f64, p1y: f64, p2y: f64) -> f64 {
    let t2 = t * t;
    let mt = 1.0 - t;
    3.0 * mt * mt * p1y + 6.0 * mt * t * (p2y - p1y) + 3.0 * t2 * (1.0 - p2y)
}

/// Compute cubic bezier value using Newton-Raphson iteration
fn cubic_bezier_value(t: f64, p1x: f64, p1y: f64, p2x: f64, p2y: f64) -> f64 {
    let epsilon = 0.0001;
    let mut x = t;

    for _ in 0..10 {
        let x_value = bezier_x(x, p1x, p2x);
        let diff = x_value - t;
        if diff.abs() < epsilon {
            break;
        }
        let derivative = bezier_x_derivative(x, p1x, p2x);
        if derivative.abs() < epsilon {
            break;
        }
        x -= diff / derivative;
    }

    bezier_y(x, p1y, p2y)
}

fn cubic_bezier_derivative(t: f64, p1x: f64, p1y: f64, p2x: f64, p2y: f64) -> f64 {
    let epsilon = 0.0001;
    let mut x = t;

    for _ in 0..10 {
        let x_value = bezier_x(x, p1x, p2x);
        let diff = x_value - t;
        if diff.abs() < epsilon {
            break;
        }
        let dx = bezier_x_derivative(x, p1x, p2x);
        if dx.abs() < epsilon {
            break;
        }
        x -= diff / dx;
    }

    let dy_dx = bezier_y_derivative(x, p1y, p2y);
    let dx_dt = bezier_x_derivative(x, p1x, p2x);

    if dx_dt.abs() < epsilon {
        return 1.0;
    }
    dy_dx / dx_dt
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_linear() {
        let e = EasingCurve::Linear;
        assert!((e.apply(0.0, 1.0) - 0.0).abs() < 1e-10);
        assert!((e.apply(0.5, 1.0) - 0.5).abs() < 1e-10);
        assert!((e.apply(1.0, 1.0) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_ease_in() {
        let e = EasingCurve::EaseIn;
        assert!((e.apply(0.0, 1.0)).abs() < 1e-10);
        assert!((e.apply(0.5, 1.0) - 0.25).abs() < 1e-10);
        assert!((e.apply(1.0, 1.0) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_ease_out() {
        let e = EasingCurve::EaseOut;
        assert!((e.apply(0.0, 1.0)).abs() < 1e-10);
        assert!((e.apply(0.5, 1.0) - 0.75).abs() < 1e-10);
        assert!((e.apply(1.0, 1.0) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_ease_in_out() {
        let e = EasingCurve::EaseInOut;
        assert!((e.apply(0.0, 1.0)).abs() < 1e-10);
        assert!((e.apply(0.5, 1.0) - 0.5).abs() < 1e-10);
        assert!((e.apply(1.0, 1.0) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_cubic_bezier_endpoints() {
        let e = EasingCurve::css_ease();
        assert!(e.apply(0.0, 1.0) < 0.01);
        assert!((e.apply(1.0, 1.0) - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_spring_endpoints() {
        let e = EasingCurve::spring_default();
        assert!(e.apply(0.0, 1.0) < 0.01);
        assert!((e.apply(1.0, 1.0) - 1.0).abs() < 0.05);
    }

    #[test]
    fn test_derivative_linear() {
        let e = EasingCurve::Linear;
        assert!((e.derivative(0.5, 1.0) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_serde_roundtrip() {
        let curves = vec![
            EasingCurve::Linear,
            EasingCurve::EaseIn,
            EasingCurve::css_ease(),
            EasingCurve::spring_default(),
        ];
        for curve in curves {
            let json = serde_json::to_string(&curve).unwrap();
            let decoded: EasingCurve = serde_json::from_str(&json).unwrap();
            assert_eq!(curve, decoded);
        }
    }
}
