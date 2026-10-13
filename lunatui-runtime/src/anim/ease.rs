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
        }
    }
}
