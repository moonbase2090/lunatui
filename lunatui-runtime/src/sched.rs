// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::time::Duration;

/// Scheduler mode per PRD §2.10.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    /// Blocked on input with no animation timer.
    Idle,
    /// Render on the next opportunity after coalescing input.
    Dirty,
    /// Deadline-driven frames at `frame_period`.
    Animating,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DirtyResume {
    Idle,
    Animating,
}

/// What the host loop should do after `Scheduler::poll`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SchedulePoll {
    /// Wait for input without scheduling a timer wakeup.
    WaitInput,
    /// Draw and present a frame now.
    Render,
    /// Sleep until this instant on the scheduler clock.
    SleepUntil(Duration),
}

/// Event-driven frame scheduler with injectable time (PRD §2.10, roadmap PR 22).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Scheduler {
    state: State,
    frame_period: Duration,
    next_frame_at: Option<Duration>,
    timer_wakeups: u64,
    dropped_frames: u64,
    last_now: Duration,
    dirty_resume: Option<DirtyResume>,
}

impl Scheduler {
    /// Starts in [`State::Animating`] with the first frame due immediately.
    #[must_use]
    pub fn animating(frame_period: Duration) -> Self {
        Self {
            state: State::Animating,
            frame_period,
            next_frame_at: Some(Duration::ZERO),
            timer_wakeups: 0,
            dropped_frames: 0,
            last_now: Duration::ZERO,
            dirty_resume: None,
        }
    }

    /// Starts blocked on input with no pending animation deadline.
    #[must_use]
    pub fn idle() -> Self {
        Self {
            state: State::Idle,
            frame_period: Duration::from_millis(16),
            next_frame_at: None,
            timer_wakeups: 0,
            dropped_frames: 0,
            last_now: Duration::ZERO,
            dirty_resume: None,
        }
    }

    fn note_dirty(&mut self) {
        if self.state != State::Dirty {
            self.dirty_resume = Some(match self.state {
                State::Animating => DirtyResume::Animating,
                State::Idle | State::Dirty => DirtyResume::Idle,
            });
        }
        self.state = State::Dirty;
    }

    fn settle_after_dirty_present(&mut self, now: Duration, write_duration: Duration) {
        let resume = self.dirty_resume.unwrap_or(DirtyResume::Idle);
        self.dirty_resume = None;
        match resume {
            DirtyResume::Idle => {
                self.state = State::Idle;
                self.next_frame_at = None;
            }
            DirtyResume::Animating => {
                self.state = State::Animating;
                self.apply_animation_backpressure(now, write_duration);
            }
        }
    }

    fn apply_animation_backpressure(&mut self, now: Duration, write_duration: Duration) {
        let finished = now.saturating_add(write_duration);
        let deadline = self.next_frame_at.unwrap_or(finished);
        if finished <= deadline {
            return;
        }
        let late = finished.saturating_sub(deadline);
        let period = self.frame_period.as_nanos();
        if period == 0 {
            return;
        }
        let skipped = late.as_nanos() / period;
        if skipped > 0 {
            self.dropped_frames += u64::try_from(skipped).unwrap_or(u64::MAX);
        }
        self.next_frame_at = Some(finished.saturating_add(self.frame_period));
    }

    /// Timer wakeups requested by [`SchedulePoll::SleepUntil`].
    #[must_use]
    pub const fn timer_wakeups(&self) -> u64 {
        self.timer_wakeups
    }

    /// Animation frames skipped after a slow present (backpressure).
    #[must_use]
    pub const fn dropped_frames(&self) -> u64 {
        self.dropped_frames
    }

    /// Current mode.
    #[must_use]
    pub const fn state(&self) -> State {
        self.state
    }

    /// Input arrived; coalesce into a dirty frame.
    pub fn on_input(&mut self) {
        self.note_dirty();
    }

    /// Resume animation ticks at `now`.
    pub fn resume_animation(&mut self, now: Duration) {
        self.state = State::Animating;
        self.next_frame_at = Some(now);
        self.last_now = now;
    }

    /// Stop animation timers and wait for input.
    pub fn settle_idle(&mut self) {
        self.state = State::Idle;
        self.next_frame_at = None;
    }

    /// Decide the next host action at `now`.
    #[must_use]
    pub fn poll(&mut self, now: Duration, input_available: bool) -> SchedulePoll {
        self.last_now = now;
        if input_available {
            self.note_dirty();
        }
        match self.state {
            State::Idle => SchedulePoll::WaitInput,
            State::Dirty => SchedulePoll::Render,
            State::Animating => {
                let deadline = self.next_frame_at.unwrap_or(now);
                if now >= deadline {
                    self.next_frame_at = Some(now.saturating_add(self.frame_period));
                    return SchedulePoll::Render;
                }
                self.timer_wakeups += 1;
                SchedulePoll::SleepUntil(deadline)
            }
        }
    }

    /// Record a completed present; applies backpressure while animating.
    pub fn after_present(&mut self, now: Duration, write_duration: Duration) {
        self.last_now = now;
        if self.state == State::Dirty {
            self.settle_after_dirty_present(now, write_duration);
            return;
        }
        if self.state == State::Animating {
            self.apply_animation_backpressure(now, write_duration);
        }
    }
}
