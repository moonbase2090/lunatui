// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![cfg(feature = "app")]
#![doc = include_str!("../README.md")]

use lunatui_core::Rect;
use lunatui_runtime::{
    AccessibilityRole as Role, Agent, ControlMetadata, ControlQuery, ControlState, HitId, HitMap,
};

fn frame(controls: &[(u32, &str, Option<&str>)]) -> HitMap {
    let mut hits = HitMap::new();
    for (row, (id, name, value)) in controls.iter().enumerate() {
        let mut metadata = ControlMetadata::new(*name, Role::TextInput);
        if let Some(value) = value {
            metadata = metadata.value(*value);
        }
        let y = u16::try_from(row).unwrap_or(0);
        hits.push_control(HitId(*id), Rect::new(0, y, 10, 1), 0, metadata);
    }
    hits
}

#[test]
fn controls_that_appear_or_go_away_are_reported_with_no_previous_or_current_value() {
    let mut subscription = Agent::subscribe(ControlQuery::kind(Role::TextInput));
    assert_eq!(
        subscription.observe(&frame(&[(1, "Name", Some("Ada"))]), None),
        Vec::new()
    );
    let changes = subscription.observe(&frame(&[(2, "Email", Some("a@b"))]), None);
    let summary: Vec<_> = changes
        .iter()
        .map(|change| {
            (
                change.id,
                change.previous.is_some(),
                change.current.is_some(),
            )
        })
        .collect();
    assert_eq!(
        summary,
        vec![(HitId(2), false, true), (HitId(1), true, false)],
        "Email appeared, Name went away"
    );
    assert_eq!(
        changes[1].name, "Name",
        "a control that went away keeps its last name"
    );
}

#[test]
fn focus_moving_between_controls_is_a_change_of_their_states() {
    let mut subscription = Agent::subscribe(ControlQuery::kind(Role::TextInput));
    let hits = frame(&[(1, "Name", None), (2, "Email", None)]);
    assert_eq!(subscription.observe(&hits, Some(HitId(1))), Vec::new());
    let changes = subscription.observe(&hits, Some(HitId(2)));
    let states: Vec<_> = changes
        .iter()
        .map(|change| {
            (
                change.id,
                change.current.as_ref().map(|value| value.states.clone()),
            )
        })
        .collect();
    assert_eq!(
        states,
        vec![
            (HitId(1), Some(Vec::new())),
            (HitId(2), Some(vec![ControlState::Focused])),
        ]
    );
}
