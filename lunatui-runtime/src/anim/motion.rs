// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use std::time::Duration;

use super::ease::Ease;

/// Micro motion: hover and focus (design standard §3).
pub const MICRO: Duration = Duration::from_millis(80);

/// Standard motion: a value change.
pub const STANDARD: Duration = Duration::from_millis(160);

/// Emphasis motion: open and close.
pub const EMPHASIS: Duration = Duration::from_millis(240);

/// No animation lasts longer than this.
pub const MAX_DURATION: Duration = Duration::from_millis(300);

const REDUCED_MOTION_VAR: &str = "LUNATUI_REDUCED_MOTION";
const SETTLE_CELLS: f32 = 0.125;
const FRAME_BUDGET: Duration = Duration::from_millis(16);
const MISSES_TO_DEGRADE: u32 = 2;
const FRAMES_TO_RECOVER: u32 = 60;
const FULL_FPS: u64 = 60;
const DEGRADED_FPS: u64 = 30;

/// Whether animations play or snap, from the `LUNATUI_REDUCED_MOTION` flag.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MotionPolicy {
    reduced: bool,
}

impl MotionPolicy {
    /// A policy that plays animations (`false`) or snaps them (`true`).
    #[must_use]
    pub const fn new(reduced: bool) -> Self {
        Self { reduced }
    }

    /// Reads `LUNATUI_REDUCED_MOTION`: set to anything but empty, `0` or `false` means reduced.
    #[must_use]
    pub fn from_env() -> Self {
        Self::from_value(std::env::var(REDUCED_MOTION_VAR).ok().as_deref())
    }

    /// Interprets a flag value the way [`from_env`](Self::from_env) does.
    #[must_use]
    pub fn from_value(value: Option<&str>) -> Self {
        let reduced = value.is_some_and(|text| {
            !(text.is_empty() || text == "0" || text.eq_ignore_ascii_case("false"))
        });
        Self { reduced }
    }

    /// Whether animations should snap to their end.
    #[must_use]
    pub const fn is_reduced(self) -> bool {
        self.reduced
    }

    /// The duration to animate over: zero when reduced, otherwise `requested` capped at
    /// [`MAX_DURATION`].
    #[must_use]
    pub fn duration(self, requested: Duration) -> Duration {
        if self.reduced {
            Duration::ZERO
        } else {
            requested.min(MAX_DURATION)
        }
    }
}

/// A value moving toward a target that can be retargeted mid-flight without a jump.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transition {
    from: f32,
    to: f32,
    start: Duration,
    duration: Duration,
    ease: Ease,
}

impl Transition {
    /// A transition at rest on `value`.
    #[must_use]
    pub const fn at_rest(value: f32) -> Self {
        Self {
            from: value,
            to: value,
            start: Duration::ZERO,
            duration: Duration::ZERO,
            ease: Ease::Linear,
        }
    }

    /// Starts moving from the current value at `now` toward `to`.
    pub fn retarget(&mut self, now: Duration, to: f32, duration: Duration, ease: Ease) {
        let from = self.value(now);
        *self = Self {
            from,
            to,
            start: now,
            duration,
            ease,
        };
    }

    /// Like [`retarget`](Self::retarget), with the duration chosen by `policy`.
    pub fn retarget_with(
        &mut self,
        policy: MotionPolicy,
        now: Duration,
        to: f32,
        duration: Duration,
        ease: Ease,
    ) {
        self.retarget(now, to, policy.duration(duration), ease);
    }

    /// The value at `now`.
    #[must_use]
    pub fn value(&self, now: Duration) -> f32 {
        let progress = if self.duration.is_zero() {
            1.0
        } else {
            let elapsed = now.saturating_sub(self.start);
            (elapsed.as_secs_f32() / self.duration.as_secs_f32()).clamp(0.0, 1.0)
        };
        self.from + (self.to - self.from) * self.ease.apply(progress)
    }

    /// The value the transition is heading for.
    #[must_use]
    pub const fn target(&self) -> f32 {
        self.to
    }

    /// Whether the value is still changing at `now`, so the app keeps scheduling frames.
    #[must_use]
    pub fn is_running(&self, now: Duration) -> bool {
        now.saturating_sub(self.start) < self.duration
    }
}

/// A spring with explicit stiffness, damping and mass, solved exactly for any step size.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DampedSpring {
    value: f32,
    velocity: f32,
    target: f32,
    omega: f32,
    zeta: f32,
    settle: f32,
}

impl DampedSpring {
    /// A spring at rest on `initial`. Non-positive stiffness or mass gives a spring that never
    /// moves.
    #[must_use]
    pub fn new(initial: f32, stiffness: f32, damping: f32, mass: f32) -> Self {
        let valid = stiffness > 0.0 && mass > 0.0;
        let omega = if valid {
            (stiffness / mass).sqrt()
        } else {
            0.0
        };
        let zeta = if valid {
            damping.max(0.0) / (2.0 * (stiffness * mass).sqrt())
        } else {
            1.0
        };
        Self {
            value: initial,
            velocity: 0.0,
            target: initial,
            omega,
            zeta,
            settle: 1e-4,
        }
    }

    /// The design standard's spring: stiffness 300, damping 30, mass 1, settled within an
    /// eighth of a cell.
    #[must_use]
    pub fn standard(initial: f32) -> Self {
        Self::new(initial, 300.0, 30.0, 1.0).with_settle(SETTLE_CELLS)
    }

    /// Sets how close to the target, in value units, counts as settled.
    #[must_use]
    pub const fn with_settle(mut self, settle: f32) -> Self {
        self.settle = settle;
        self
    }

    /// Current value.
    #[must_use]
    pub const fn value(&self) -> f32 {
        self.value
    }

    /// Current velocity in value units per second.
    #[must_use]
    pub const fn velocity(&self) -> f32 {
        self.velocity
    }

    /// Target value.
    #[must_use]
    pub const fn target(&self) -> f32 {
        self.target
    }

    /// Retargets the spring; velocity carries over, so interruption does not jump.
    pub fn set_target(&mut self, target: f32) {
        self.target = target;
    }

    /// Jumps to the target and stops, for reduced motion.
    pub fn snap(&mut self) {
        self.value = self.target;
        self.velocity = 0.0;
    }

    /// Advances by `dt` with the exact solution for the spring's damping regime.
    pub fn advance(&mut self, dt: Duration) {
        let secs = dt.as_secs_f32();
        if secs <= 0.0 || self.omega == 0.0 {
            return;
        }
        let omega = self.omega;
        let zeta = self.zeta;
        let offset = self.value - self.target;
        let speed = self.velocity;
        let (next_offset, next_speed) = if zeta < 1.0 - 1e-4 {
            let damped = omega * (1.0 - zeta * zeta).sqrt();
            let decay = (-zeta * omega * secs).exp();
            let (sin, cos) = (damped * secs).sin_cos();
            let sin_gain = (speed + zeta * omega * offset) / damped;
            (
                decay * (offset * cos + sin_gain * sin),
                decay
                    * ((sin_gain * damped - zeta * omega * offset) * cos
                        - (offset * damped + zeta * omega * sin_gain) * sin),
            )
        } else if zeta <= 1.0 + 1e-4 {
            let slope = speed + omega * offset;
            let decay = (-omega * secs).exp();
            (
                (offset + slope * secs) * decay,
                (slope - omega * (offset + slope * secs)) * decay,
            )
        } else {
            let root = (zeta * zeta - 1.0).sqrt();
            let slow = -omega * (zeta - root);
            let fast = -omega * (zeta + root);
            let slow_gain = (speed - fast * offset) / (slow - fast);
            let fast_gain = offset - slow_gain;
            let slow_decay = (slow * secs).exp();
            let fast_decay = (fast * secs).exp();
            (
                slow_gain * slow_decay + fast_gain * fast_decay,
                slow_gain * slow * slow_decay + fast_gain * fast * fast_decay,
            )
        };
        self.value = self.target + next_offset;
        self.velocity = next_speed;
    }

    /// Whether the spring is within its settle distance of the target and nearly still.
    #[must_use]
    pub fn is_settled(&self) -> bool {
        (self.value - self.target).abs() < self.settle && self.velocity.abs() < self.settle
    }
}

/// Watches frame times and drops from 60 to 30 fps after two slow frames in a row.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FramePacer {
    misses: u32,
    good: u32,
    degraded: bool,
}

impl FramePacer {
    /// A pacer at 60 fps.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            misses: 0,
            good: 0,
            degraded: false,
        }
    }

    /// Records how long a frame took, including diff and write.
    ///
    /// Two consecutive frames over the 16 ms budget degrade to 30 fps; 60 consecutive frames
    /// within budget restore 60 fps.
    pub fn record(&mut self, frame_time: Duration) {
        if frame_time > FRAME_BUDGET {
            self.misses += 1;
            self.good = 0;
            if self.misses >= MISSES_TO_DEGRADE {
                self.degraded = true;
            }
        } else {
            self.misses = 0;
            self.good += 1;
            if self.degraded && self.good >= FRAMES_TO_RECOVER {
                self.degraded = false;
            }
        }
    }

    /// Whether the pacer is running at the reduced rate.
    #[must_use]
    pub const fn is_degraded(&self) -> bool {
        self.degraded
    }

    /// Frames per second to aim for.
    #[must_use]
    pub const fn fps(&self) -> u64 {
        if self.degraded {
            DEGRADED_FPS
        } else {
            FULL_FPS
        }
    }

    /// Time between frames at the current rate.
    #[must_use]
    pub const fn frame_period(&self) -> Duration {
        Duration::from_nanos(1_000_000_000 / self.fps())
    }
}
