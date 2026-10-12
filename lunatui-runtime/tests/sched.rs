// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

#![doc = include_str!("../README.md")]

use std::time::Duration;

use lunatui_runtime::{SchedulePoll, Scheduler, State};

const FRAME: Duration = Duration::from_millis(16);

#[test]
fn idle_poll_waits_without_timer_wakeups() {
    let mut sched = Scheduler::idle();
    assert_eq!(sched.poll(Duration::ZERO, false), SchedulePoll::WaitInput);
    assert_eq!(sched.timer_wakeups(), 0);
    assert_eq!(sched.state(), State::Idle);
}

#[test]
fn idle_poll_renders_when_input_arrives() {
    let mut sched = Scheduler::idle();
    assert_eq!(sched.poll(Duration::ZERO, true), SchedulePoll::Render);
    assert_eq!(sched.state(), State::Dirty);
}

#[test]
fn animating_poll_sleeps_until_the_frame_deadline() {
    let mut sched = Scheduler::animating(FRAME);
    assert_eq!(sched.poll(Duration::ZERO, false), SchedulePoll::Render);
    assert_eq!(
        sched.poll(Duration::from_millis(5), false),
        SchedulePoll::SleepUntil(FRAME)
    );
    assert_eq!(sched.timer_wakeups(), 1);
}

#[test]
fn dirty_poll_always_renders() {
    let mut sched = Scheduler::idle();
    sched.on_input();
    assert_eq!(sched.poll(Duration::ZERO, false), SchedulePoll::Render);
}

#[test]
fn dirty_settles_to_idle_after_present() {
    let mut sched = Scheduler::idle();
    assert_eq!(sched.poll(Duration::ZERO, true), SchedulePoll::Render);
    sched.after_present(Duration::ZERO, Duration::ZERO);
    assert_eq!(sched.state(), State::Idle);
    assert_eq!(sched.poll(Duration::ZERO, false), SchedulePoll::WaitInput);
    assert_eq!(sched.poll(Duration::ZERO, false), SchedulePoll::WaitInput);
}

#[test]
fn dirty_during_animation_resumes_animating() {
    let mut sched = Scheduler::animating(FRAME);
    assert_eq!(sched.poll(Duration::ZERO, false), SchedulePoll::Render);
    sched.after_present(Duration::ZERO, Duration::ZERO);
    assert_eq!(sched.poll(Duration::from_millis(5), true), SchedulePoll::Render);
    sched.after_present(Duration::from_millis(5), Duration::ZERO);
    assert_eq!(sched.state(), State::Animating);
    assert_eq!(
        sched.poll(Duration::from_millis(5), false),
        SchedulePoll::SleepUntil(FRAME)
    );
}

#[test]
fn slow_present_drops_intermediate_animation_frames() {
    let mut sched = Scheduler::animating(FRAME);
    assert_eq!(sched.poll(Duration::ZERO, false), SchedulePoll::Render);
    sched.after_present(Duration::ZERO, FRAME.saturating_mul(3));
    assert!(sched.dropped_frames() >= 2);
}
