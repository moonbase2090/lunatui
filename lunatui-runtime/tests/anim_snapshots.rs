// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use std::time::Duration;

use lunatui_core::{Color, Rect};
use lunatui_runtime::{Clock, Ease, Spring, Tween};

#[test]
fn tween_f32_snapshot_at_half_duration() {
    let tween = Tween::new(0.0, 100.0, Duration::from_secs(1), Ease::Linear);
    let mid = tween.sample(Duration::from_millis(500));
    assert!((mid - 50.0).abs() < 0.001);
    assert_eq!(tween.sample(Duration::ZERO), 0.0);
    assert_eq!(tween.sample(Duration::from_secs(2)), 100.0);
}

#[test]
fn spring_advances_independently_of_step_size() {
    let mut one_step = Spring::new(0.0);
    one_step.set_target(1.0);
    one_step.advance(Duration::from_millis(100));

    let mut many_steps = Spring::new(0.0);
    many_steps.set_target(1.0);
    for _ in 0..10 {
        many_steps.advance(Duration::from_millis(10));
    }

    assert!((one_step.value() - many_steps.value()).abs() < 1e-4);
}

#[test]
fn rect_tween_snapshot_at_time_t() {
    let from = Rect::new(0, 0, 10, 4);
    let to = Rect::new(10, 8, 20, 12);
    let tween = Tween::new(from, to, Duration::from_secs(1), Ease::Linear);
    let mid = tween.sample(Duration::from_millis(500));
    assert_eq!(mid, Rect::new(5, 4, 15, 8));
}

#[test]
fn color_tween_blends_rgb_in_oklab_at_midpoint() {
    let from = Color::Rgb(0, 0, 0);
    let to = Color::Rgb(255, 255, 255);
    let tween = Tween::new(from, to, Duration::from_secs(1), Ease::Linear);
    let mid = tween.sample(Duration::from_millis(500));
    assert_eq!(mid, Color::Rgb(145, 86, 0));
}

#[test]
fn clock_advances_deterministically() {
    let mut clock = Clock::new();
    assert_eq!(clock.now(), Duration::ZERO);
    clock.advance(Duration::from_millis(16));
    clock.advance(Duration::from_millis(16));
    assert_eq!(clock.now(), Duration::from_millis(32));
}
