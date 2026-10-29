// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::Rect;
use lunatui_runtime::{HitId, HitMap};
use lunatui_widgets::Toggle;

const FIRST: HitId = HitId(30);
const LABELS: [&str; 3] = ["Dark mode", "Alerts", "Sync"];

fn switches(state: [bool; 3]) -> Vec<Toggle> {
    LABELS
        .iter()
        .zip(state)
        .map(|(label, on)| Toggle::new(on).label(*label))
        .collect()
}

fn map(state: [bool; 3]) -> HitMap {
    let mut map = HitMap::new();
    let areas = switches(state)
        .iter()
        .enumerate()
        .map(|(index, toggle)| {
            let y = 2 + u16::try_from(index).unwrap_or(0) * 2;
            (index, toggle.hit_area(Rect::new(4, y, 30, 1)))
        })
        .collect::<Vec<_>>();
    map.push_indexed(FIRST, areas, 0);
    map
}

fn click(state: &mut [bool; 3], column: u16, row: u16) -> Option<usize> {
    let index = map(*state).pick(column, row)?.index_in(FIRST, 3)?;
    state[index] = !state[index];
    Some(index)
}

#[test]
fn a_click_on_the_switch_or_its_label_flips_that_switch() {
    let mut state = [false, true, false];
    assert_eq!(click(&mut state, 4, 2), Some(0));
    assert_eq!(state, [true, true, false]);
    assert_eq!(click(&mut state, 12, 2), Some(0));
    assert_eq!(state, [false, true, false]);
    assert_eq!(click(&mut state, 9, 4), Some(1));
    assert_eq!(state, [false, false, false]);
    assert_eq!(click(&mut state, 14, 6), None);
}

#[test]
fn clicks_beside_the_switches_flip_nothing() {
    let mut state = [false; 3];
    assert_eq!(click(&mut state, 3, 2), None);
    assert_eq!(click(&mut state, 4 + 4 + 1 + 9, 2), None);
    assert_eq!(click(&mut state, 5, 3), None);
    assert_eq!(click(&mut state, 5, 1), None);
    assert_eq!(state, [false; 3]);
}

#[test]
fn the_label_is_part_of_the_target_to_its_last_cell() {
    let mut state = [false; 3];
    assert_eq!(click(&mut state, 4 + 4 + 1 + 8, 2), Some(0));
    assert_eq!(click(&mut state, 4 + 4 + 1 + 9, 2), None);
    assert_eq!(click(&mut state, 4 + 3, 6), Some(2));
    assert_eq!(click(&mut state, 4 + 4 + 1 + 4, 6), None);
}

#[test]
fn a_dialog_over_the_switches_takes_the_click() {
    let state = [false; 3];
    let mut layered = map(state);
    layered.push(HitId(900), Rect::new(2, 1, 20, 4), 1);
    assert_eq!(layered.pick(6, 2), Some(HitId(900)));
    assert_eq!(layered.pick(6, 6), Some(HitId(32)));
}
