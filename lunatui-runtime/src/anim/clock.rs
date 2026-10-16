// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use std::time::Duration;

/// Injectable monotonic clock for deterministic animation tests (PRD §2.10).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Clock {
    now: Duration,
}

impl Clock {
    /// Starts at zero.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            now: Duration::ZERO,
        }
    }

    /// Starts at `time`.
    #[must_use]
    pub const fn at(time: Duration) -> Self {
        Self { now: time }
    }

    /// Current time.
    #[must_use]
    pub const fn now(&self) -> Duration {
        self.now
    }

    /// Moves time forward.
    pub fn advance(&mut self, delta: Duration) {
        self.now += delta;
    }

    /// Sets the current time.
    pub fn set(&mut self, time: Duration) {
        self.now = time;
    }
}

impl Default for Clock {
    fn default() -> Self {
        Self::new()
    }
}
