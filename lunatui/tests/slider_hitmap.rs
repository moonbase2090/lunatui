// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::Rect;
use lunatui_runtime::{HitId, HitMap};
use lunatui_widgets::Slider;

const TRACK: HitId = HitId(60);
const AREA: Rect = Rect::new(4, 2, 30, 1);

fn slider(value: f64) -> Slider<'static> {
    Slider::new(0.0, 100.0)
        .step(5.0)
        .label("Volume")
        .value(value)
}

fn map(slider: &Slider<'_>) -> HitMap {
    let mut map = HitMap::new();
    map.push(TRACK, slider.track_area(AREA), 0);
    map
}

fn drag_to(slider: &Slider<'_>, column: u16, row: u16) -> Option<f64> {
    let hits = map(slider);
    (hits.pick(column, row)? == TRACK).then(|| slider.value_at(slider.track_area(AREA), column))
}

#[test]
fn a_click_on_the_track_sets_the_value_under_it() {
    let slider = slider(20.0);
    let track = slider.track_area(AREA);
    assert_eq!(track, Rect::new(11, 2, 23, 1));
    assert_eq!(drag_to(&slider, 11, 2), Some(0.0));
    assert_eq!(drag_to(&slider, 33, 2), Some(100.0));
    let middle = drag_to(&slider, 22, 2);
    assert_eq!(middle, Some(50.0));
}

#[test]
fn clicks_on_the_label_or_off_the_row_reach_nothing() {
    let slider = slider(20.0);
    assert_eq!(drag_to(&slider, 5, 2), None);
    assert_eq!(drag_to(&slider, 10, 2), None);
    assert_eq!(drag_to(&slider, 20, 1), None);
    assert_eq!(drag_to(&slider, 20, 3), None);
    assert_eq!(drag_to(&slider, 34, 2), None);
}

#[test]
fn a_drag_walks_the_value_across_the_track_in_steps() {
    let mut value = 0.0;
    let mut seen = Vec::new();
    for column in 11..=33 {
        if let Some(next) = drag_to(&slider(value), column, 2) {
            value = next;
            seen.push(next);
        }
    }
    assert!(seen.windows(2).all(|pair| pair[0] <= pair[1]));
    assert!(seen.iter().all(|value| (value / 5.0).fract().abs() < 1e-9));
    assert_eq!(seen.first(), Some(&0.0));
    assert_eq!(seen.last(), Some(&100.0));
}

#[test]
fn the_thumb_cell_follows_the_value_and_arrow_keys_nudge_it() {
    let low = slider(20.0);
    let thumbs = low.thumb_areas(AREA);
    assert_eq!(thumbs.len(), 1);
    let before = thumbs[0].1.x;
    let moved = slider(low.nudge(20.0, 6));
    assert!((low.nudge(20.0, 6) - 50.0).abs() < 1e-9);
    assert!(moved.thumb_areas(AREA)[0].1.x > before);
    assert!((low.nudge(20.0, -9) - 0.0).abs() < 1e-9);
}

#[test]
fn a_dialog_over_the_slider_takes_the_click() {
    let slider = slider(20.0);
    let mut hits = map(&slider);
    hits.push(HitId(900), Rect::new(10, 1, 10, 3), 1);
    assert_eq!(hits.pick(15, 2), Some(HitId(900)));
    assert_eq!(hits.pick(25, 2), Some(TRACK));
}
