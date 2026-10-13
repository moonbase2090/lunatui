// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use std::time::Duration;

const SETTLED_EPSILON: f32 = 1e-4;

/// Critically damped spring toward a target (PRD §2.10).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Spring {
    value: f32,
    velocity: f32,
    target: f32,
    omega: f32,
}

impl Spring {
    /// Starts at `initial` with a default stiffness (`omega = 12`).
    #[must_use]
    pub fn new(initial: f32) -> Self {
        Self::with_omega(initial, 12.0)
    }

    /// Starts at `initial` with the given `omega` (`sqrt(stiffness)`).
    #[must_use]
    pub fn with_omega(initial: f32, omega: f32) -> Self {
        Self {
            value: initial,
            velocity: 0.0,
            target: initial,
            omega: omega.max(0.0),
        }
    }

    /// Current value.
    #[must_use]
    pub const fn value(&self) -> f32 {
        self.value
    }

    /// Target value.
    #[must_use]
    pub const fn target(&self) -> f32 {
        self.target
    }

    /// Sets the target without resetting velocity.
    pub fn set_target(&mut self, target: f32) {
        self.target = target;
    }

    /// Advances by `dt` with an exact critically damped integrator.
    pub fn advance(&mut self, dt: Duration) {
        let dt_s = dt.as_secs_f32();
        if dt_s <= 0.0 || self.omega == 0.0 {
            return;
        }
        let w = self.omega;
        let y = self.value - self.target;
        let c1 = y;
        let c2 = self.velocity + w * y;
        let exp = (-w * dt_s).exp();
        let y_new = (c1 + c2 * dt_s) * exp;
        let v_new = (c2 - w * (c1 + c2 * dt_s)) * exp;
        self.value = self.target + y_new;
        self.velocity = v_new;
    }

    /// Returns true when the spring is within `SETTLED_EPSILON` of the target.
    #[must_use]
    pub fn is_settled(&self) -> bool {
        (self.value - self.target).abs() < SETTLED_EPSILON && self.velocity.abs() < SETTLED_EPSILON
    }
}
