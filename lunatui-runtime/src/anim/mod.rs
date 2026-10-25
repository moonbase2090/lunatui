// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

mod clock;
mod ease;
mod spring;
mod tween;

pub use clock::Clock;
pub use ease::Ease;
pub use spring::Spring;
pub use tween::Tween;

#[cfg(feature = "motion")]
mod motion;

#[cfg(feature = "motion")]
pub use motion::{
    DampedSpring, EMPHASIS, FramePacer, MAX_DURATION, MICRO, MotionPolicy, STANDARD, Transition,
};
