// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use std::time::Duration;

use lunatui_core::Rect;
use lunatui_runtime::{Clock, HitId, HitMap};
use lunatui_widgets::{ToastId, ToastKind, ToastScope, ToastStack};

const AREA: Rect = Rect::new(0, 0, 40, 12);
const TOASTS: u32 = 1000;
const PANE_HIT: HitId = HitId(7);

fn pane(id: u32) -> Option<Rect> {
    (id == 1).then_some(Rect::new(0, 0, 40, 12))
}

fn hits(stack: &ToastStack, clock: &Clock) -> HitMap {
    let mut map = HitMap::new();
    map.push(PANE_HIT, AREA, 0);
    for placement in stack.placements(clock.now(), AREA, pane) {
        map.push(HitId(TOASTS + placement.id.0), placement.rect, placement.z);
    }
    map
}

fn toast_at(map: &HitMap, column: u16, row: u16) -> Option<ToastId> {
    let id = map.pick(column, row)?;
    id.0.checked_sub(TOASTS).map(ToastId)
}

#[test]
fn a_click_reaches_the_toast_drawn_under_it_and_the_pane_elsewhere() {
    let clock = Clock::new();
    let mut stack = ToastStack::new();
    let id = stack.push(
        clock.now(),
        Some(Duration::from_secs(4)),
        ToastScope::Pane(1),
        ToastKind::Info,
        "Saved",
    );
    let map = hits(&stack, &clock);
    assert_eq!(toast_at(&map, 31, 9), Some(id));
    assert_eq!(toast_at(&map, 39, 11), Some(id));
    assert_eq!(toast_at(&map, 30, 9), None);
    assert_eq!(toast_at(&map, 31, 8), None);
    assert_eq!(map.pick(30, 9), Some(PANE_HIT));
}

#[test]
fn a_toast_stops_catching_clicks_when_the_virtual_clock_reaches_its_time() {
    let mut clock = Clock::new();
    let mut stack = ToastStack::new();
    let id = stack.push(
        clock.now(),
        Some(Duration::from_secs(4)),
        ToastScope::Pane(1),
        ToastKind::Warning,
        "Heads up",
    );
    clock.advance(Duration::from_millis(3999));
    assert_eq!(toast_at(&hits(&stack, &clock), 39, 11), Some(id));
    clock.advance(Duration::from_millis(1));
    assert_eq!(toast_at(&hits(&stack, &clock), 39, 11), None);
    assert_eq!(stack.expire(clock.now()), [id]);
    assert_eq!(hits(&stack, &clock).pick(39, 11), Some(PANE_HIT));
}

#[test]
fn overlapping_toasts_answer_to_the_one_on_top() {
    let clock = Clock::new();
    let mut stack = ToastStack::new();
    let pane_toast = stack.push(
        clock.now(),
        None,
        ToastScope::Pane(1),
        ToastKind::Info,
        "Pane message",
    );
    let status = stack.push(
        clock.now(),
        None,
        ToastScope::Status,
        ToastKind::Error,
        "Build failed",
    );
    let map = hits(&stack, &clock);
    assert_eq!(toast_at(&map, 39, 11), Some(status));
    assert_eq!(toast_at(&map, 39, 10), Some(pane_toast));
    assert_eq!(toast_at(&map, 39, 9), Some(pane_toast));
}

#[test]
fn a_newer_toast_is_on_top_of_an_older_one_where_they_overlap() {
    let clock = Clock::new();
    let mut stack = ToastStack::new();
    let a = stack.push(
        clock.now(),
        None,
        ToastScope::Status,
        ToastKind::Info,
        "first",
    );
    let b = stack.push(
        clock.now(),
        None,
        ToastScope::Status,
        ToastKind::Info,
        "second",
    );
    let map = hits(&stack, &clock);
    assert_eq!(toast_at(&map, 39, 11), Some(b));
    assert_eq!(toast_at(&map, 39, 10), Some(a));
}

#[test]
fn dismissing_on_click_lets_the_toast_below_take_the_spot() {
    let clock = Clock::new();
    let mut stack = ToastStack::new();
    let old = stack.push(
        clock.now(),
        None,
        ToastScope::Status,
        ToastKind::Info,
        "old",
    );
    let new = stack.push(
        clock.now(),
        None,
        ToastScope::Status,
        ToastKind::Info,
        "new",
    );
    let clicked = toast_at(&hits(&stack, &clock), 39, 11);
    assert_eq!(clicked, Some(new));
    assert!(stack.dismiss(new));
    assert_eq!(toast_at(&hits(&stack, &clock), 39, 11), Some(old));
}
