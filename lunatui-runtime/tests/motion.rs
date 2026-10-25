// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use std::time::Duration;

use lunatui_runtime::{
    DampedSpring, EMPHASIS, Ease, FramePacer, MAX_DURATION, MICRO, MotionPolicy, STANDARD,
    SchedulePoll, Scheduler, Spring, Transition, Tween,
};

fn ms(value: u64) -> Duration {
    Duration::from_millis(value)
}

#[test]
fn out_cubic_passes_through_the_hand_computed_point() {
    assert!((Ease::OutCubic.apply(0.2) - 0.5).abs() < 1e-3);
    assert_eq!(Ease::OutCubic.apply(0.0), 0.0);
    assert_eq!(Ease::OutCubic.apply(1.0), 1.0);
}

#[test]
fn in_cubic_passes_through_the_hand_computed_point() {
    assert!((Ease::InCubic.apply(0.65) - 0.5).abs() < 1e-3);
    assert_eq!(Ease::InCubic.apply(0.0), 0.0);
    assert_eq!(Ease::InCubic.apply(1.0), 1.0);
}

#[test]
fn out_cubic_leads_linear_and_in_cubic_trails_it() {
    for step in 1..10_u8 {
        let t = f32::from(step) / 10.0;
        assert!(Ease::OutCubic.apply(t) > t, "out-cubic at {t}");
        assert!(Ease::InCubic.apply(t) < t, "in-cubic at {t}");
    }
}

#[test]
fn bezier_curves_never_decrease() {
    for ease in [Ease::OutCubic, Ease::InCubic] {
        let mut last = 0.0_f32;
        for step in 0..=200_u16 {
            let value = ease.apply(f32::from(step) / 200.0);
            assert!(value + 1e-6 >= last, "{ease:?} fell at step {step}");
            last = value;
        }
    }
}

#[test]
fn durations_match_the_standard() {
    assert_eq!(MICRO, ms(80));
    assert_eq!(STANDARD, ms(160));
    assert_eq!(EMPHASIS, ms(240));
    assert_eq!(MAX_DURATION, ms(300));
}

#[test]
fn reduced_motion_flag_values() {
    for value in [None, Some(""), Some("0"), Some("false"), Some("FALSE")] {
        assert!(!MotionPolicy::from_value(value).is_reduced(), "{value:?}");
    }
    for value in [Some("1"), Some("true"), Some("yes")] {
        assert!(MotionPolicy::from_value(value).is_reduced(), "{value:?}");
    }
}

#[test]
fn policy_caps_durations_and_zeroes_them_when_reduced() {
    let normal = MotionPolicy::new(false);
    assert_eq!(normal.duration(STANDARD), ms(160));
    assert_eq!(normal.duration(Duration::from_secs(1)), ms(300));
    assert_eq!(MotionPolicy::new(true).duration(EMPHASIS), Duration::ZERO);
}

#[test]
fn a_reduced_tween_snaps_to_its_end() {
    let duration = MotionPolicy::new(true).duration(STANDARD);
    let tween = Tween::new(0.0, 10.0, duration, Ease::OutCubic);
    assert_eq!(tween.sample(Duration::ZERO), 10.0);
}

#[test]
fn retargeting_a_transition_does_not_jump() {
    let mut transition = Transition::at_rest(0.0);
    transition.retarget(ms(0), 10.0, ms(100), Ease::Linear);
    assert!((transition.value(ms(50)) - 5.0).abs() < 1e-4);
    transition.retarget(ms(50), 0.0, ms(100), Ease::Linear);
    assert!((transition.value(ms(50)) - 5.0).abs() < 1e-4);
    assert!((transition.value(ms(100)) - 2.5).abs() < 1e-4);
    assert!((transition.value(ms(150)) - 0.0).abs() < 1e-4);
}

#[test]
fn a_transition_reports_whether_it_is_running() {
    let mut transition = Transition::at_rest(1.0);
    assert!(!transition.is_running(ms(0)));
    transition.retarget(ms(10), 2.0, ms(100), Ease::Linear);
    assert!(transition.is_running(ms(10)));
    assert!(transition.is_running(ms(109)));
    assert!(!transition.is_running(ms(110)));
    assert_eq!(transition.target(), 2.0);
}

#[test]
fn a_reduced_retarget_snaps_immediately() {
    let mut transition = Transition::at_rest(0.0);
    transition.retarget_with(
        MotionPolicy::new(true),
        ms(5),
        7.0,
        STANDARD,
        Ease::OutCubic,
    );
    assert_eq!(transition.value(ms(5)), 7.0);
    assert!(!transition.is_running(ms(5)));
}

fn peak_and_settle(mut spring: DampedSpring, travel: f32) -> (f32, Option<u32>) {
    spring.set_target(travel);
    let mut peak = 0.0_f32;
    let mut settled_at = None;
    for frame in 1..=300_u32 {
        spring.advance(ms(16));
        peak = peak.max(spring.value());
        if settled_at.is_none() && spring.is_settled() {
            settled_at = Some(frame);
        }
    }
    (peak, settled_at)
}

#[test]
fn the_standard_spring_settles_quickly_without_visible_overshoot() {
    let (peak, settled_at) = peak_and_settle(DampedSpring::standard(0.0), 10.0);
    assert!(peak < 10.0 + 0.125, "peak {peak}");
    let frames = settled_at.unwrap_or(u32::MAX);
    assert!(frames <= 32, "settled after {frames} frames");
    assert!(frames >= 3, "settled after only {frames} frames");
}

#[test]
fn spring_result_does_not_depend_on_step_size() {
    let mut one = DampedSpring::standard(0.0);
    one.set_target(10.0);
    one.advance(ms(100));
    let mut many = DampedSpring::standard(0.0);
    many.set_target(10.0);
    for _ in 0..10 {
        many.advance(ms(10));
    }
    assert!((one.value() - many.value()).abs() < 1e-3);
    assert!((one.velocity() - many.velocity()).abs() < 1e-2);
}

#[test]
fn a_critically_damped_spring_matches_the_runtime_spring() {
    let mut general = DampedSpring::new(0.0, 100.0, 20.0, 1.0);
    general.set_target(1.0);
    general.advance(ms(100));
    let mut critical = Spring::with_omega(0.0, 10.0);
    critical.set_target(1.0);
    critical.advance(ms(100));
    assert!((general.value() - critical.value()).abs() < 1e-4);
}

#[test]
fn an_overdamped_spring_never_overshoots() {
    let (peak, settled_at) = peak_and_settle(
        DampedSpring::new(0.0, 100.0, 50.0, 1.0).with_settle(0.01),
        1.0,
    );
    assert!(peak <= 1.0 + 1e-4, "peak {peak}");
    assert!(settled_at.is_some());
}

#[test]
fn an_underdamped_spring_overshoots_its_target() {
    let (peak, _) = peak_and_settle(DampedSpring::new(0.0, 100.0, 2.0, 1.0), 1.0);
    assert!(peak > 1.2, "peak {peak}");
}

#[test]
fn retargeting_a_spring_keeps_its_velocity() {
    let mut spring = DampedSpring::standard(0.0);
    spring.set_target(10.0);
    spring.advance(ms(50));
    let (value, velocity) = (spring.value(), spring.velocity());
    spring.set_target(-5.0);
    assert_eq!(spring.value(), value);
    assert_eq!(spring.velocity(), velocity);
    assert!(velocity > 0.0);
}

#[test]
fn snap_jumps_to_the_target_and_stops() {
    let mut spring = DampedSpring::standard(0.0);
    spring.set_target(4.0);
    spring.advance(ms(30));
    spring.snap();
    assert_eq!(spring.value(), 4.0);
    assert_eq!(spring.velocity(), 0.0);
    assert!(spring.is_settled());
}

#[test]
fn a_spring_without_stiffness_never_moves() {
    let mut spring = DampedSpring::new(3.0, 0.0, 10.0, 1.0);
    spring.set_target(9.0);
    spring.advance(ms(500));
    assert_eq!(spring.value(), 3.0);
}

#[test]
fn the_pacer_starts_at_sixty_fps() {
    let pacer = FramePacer::new();
    assert_eq!(pacer.fps(), 60);
    assert_eq!(pacer.frame_period(), Duration::from_nanos(16_666_666));
    assert!(!pacer.is_degraded());
}

#[test]
fn two_consecutive_slow_frames_drop_to_thirty_fps() {
    let mut pacer = FramePacer::new();
    pacer.record(ms(20));
    assert!(!pacer.is_degraded());
    pacer.record(ms(20));
    assert!(pacer.is_degraded());
    assert_eq!(pacer.fps(), 30);
    assert_eq!(pacer.frame_period(), Duration::from_nanos(33_333_333));
}

#[test]
fn slow_frames_that_are_not_consecutive_do_not_degrade() {
    let mut pacer = FramePacer::new();
    for _ in 0..10 {
        pacer.record(ms(20));
        pacer.record(ms(5));
    }
    assert!(!pacer.is_degraded());
}

#[test]
fn a_frame_exactly_on_budget_is_not_a_miss() {
    let mut pacer = FramePacer::new();
    for _ in 0..5 {
        pacer.record(ms(16));
    }
    assert!(!pacer.is_degraded());
}

#[test]
fn sixty_good_frames_restore_sixty_fps() {
    let mut pacer = FramePacer::new();
    pacer.record(ms(30));
    pacer.record(ms(30));
    for _ in 0..59 {
        pacer.record(ms(4));
    }
    assert!(pacer.is_degraded());
    pacer.record(ms(4));
    assert!(!pacer.is_degraded());
}

#[test]
fn the_scheduler_follows_a_changed_frame_period() {
    let mut scheduler = Scheduler::animating(ms(16));
    assert_eq!(scheduler.poll(ms(0), false), SchedulePoll::Render);
    scheduler.set_frame_period(ms(33));
    scheduler.after_present(ms(1), Duration::ZERO);
    assert_eq!(
        scheduler.poll(ms(15), false),
        SchedulePoll::SleepUntil(ms(16))
    );
    assert_eq!(scheduler.poll(ms(16), false), SchedulePoll::Render);
    assert_eq!(
        scheduler.poll(ms(17), false),
        SchedulePoll::SleepUntil(ms(49))
    );
}
