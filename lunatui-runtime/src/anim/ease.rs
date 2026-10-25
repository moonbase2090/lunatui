// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

/// Easing curve applied to normalized progress in `[0, 1]`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Ease {
    /// `t`.
    #[default]
    Linear,
    /// Smooth ease in and out (cubic).
    InOutCubic,
    /// Cubic bezier `(0.2, 0, 0, 1)`: fast start, long settle, for values and entering.
    #[cfg(feature = "motion")]
    OutCubic,
    /// Cubic bezier `(0.4, 0, 1, 1)`: slow start, fast finish, for exiting.
    #[cfg(feature = "motion")]
    InCubic,
}

impl Ease {
    /// Maps `t` in `[0, 1]` through the curve.
    #[must_use]
    pub fn apply(self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        match self {
            Self::Linear => t,
            Self::InOutCubic => {
                if t < 0.5 {
                    4.0 * t * t * t
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
                }
            }
            #[cfg(feature = "motion")]
            Self::OutCubic => cubic_bezier(0.2, 0.0, 0.0, 1.0, t),
            #[cfg(feature = "motion")]
            Self::InCubic => cubic_bezier(0.4, 0.0, 1.0, 1.0, t),
        }
    }
}

#[cfg(feature = "motion")]
fn cubic_bezier(x1: f32, y1: f32, x2: f32, y2: f32, x: f32) -> f32 {
    if x <= 0.0 {
        return 0.0;
    }
    if x >= 1.0 {
        return 1.0;
    }
    let curve = |a: f32, b: f32, s: f32| {
        let inv = 1.0 - s;
        3.0 * inv * inv * s * a + 3.0 * inv * s * s * b + s * s * s
    };
    let slope = |a: f32, b: f32, s: f32| {
        let inv = 1.0 - s;
        3.0 * inv * inv * a + 6.0 * inv * s * (b - a) + 3.0 * s * s * (1.0 - b)
    };
    let mut s = x;
    for _ in 0..8 {
        let error = curve(x1, x2, s) - x;
        let derivative = slope(x1, x2, s);
        if error.abs() < 1e-6 || derivative.abs() < 1e-6 {
            break;
        }
        s = (s - error / derivative).clamp(0.0, 1.0);
    }
    let (mut low, mut high) = (0.0_f32, 1.0_f32);
    for _ in 0..24 {
        if (curve(x1, x2, s) - x).abs() < 1e-6 {
            break;
        }
        if curve(x1, x2, s) < x {
            low = s;
        } else {
            high = s;
        }
        s = f32::midpoint(low, high);
    }
    curve(y1, y2, s)
}
