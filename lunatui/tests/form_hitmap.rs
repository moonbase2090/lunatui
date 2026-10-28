// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::Rect;
use lunatui_runtime::{HitId, HitMap};
use lunatui_widgets::{Form, FormField};

const FIRST: HitId = HitId(50);
const AREA: Rect = Rect::new(2, 1, 40, 20);

fn form() -> Form {
    Form::new([
        FormField::new("Name").required(),
        FormField::new("Email").required().error("Required"),
        FormField::new("Notes").disabled(),
    ])
}

fn map(form: &Form) -> HitMap {
    let mut map = HitMap::new();
    for area in form.layout(AREA) {
        map.push(
            HitId(FIRST.0 + u32::try_from(area.index).unwrap_or(0)),
            area.row,
            0,
        );
    }
    map
}

fn field_at(map: &HitMap, column: u16, row: u16) -> Option<usize> {
    map.pick(column, row)?.index_in(FIRST, 3)
}

#[test]
fn a_click_anywhere_on_a_field_row_group_reaches_that_field() {
    let hits = map(&form());
    assert_eq!(field_at(&hits, 2, 1), Some(0));
    assert_eq!(field_at(&hits, 20, 1), Some(0));
    assert_eq!(field_at(&hits, 20, 2), Some(0));
    assert_eq!(field_at(&hits, 2, 4), Some(1));
    assert_eq!(field_at(&hits, 41, 5), Some(1));
    assert_eq!(field_at(&hits, 5, 7), Some(2));
}

#[test]
fn the_gap_between_fields_and_the_area_edges_reach_nothing() {
    let hits = map(&form());
    assert_eq!(field_at(&hits, 5, 3), None);
    assert_eq!(field_at(&hits, 5, 6), None);
    assert_eq!(field_at(&hits, 1, 1), None);
    assert_eq!(field_at(&hits, 42, 1), None);
    assert_eq!(field_at(&hits, 5, 9), None);
    assert_eq!(field_at(&hits, 5, 0), None);
}

#[test]
fn clicks_move_focus_but_a_disabled_field_keeps_it_out_of_the_tab_order() {
    let form = form();
    let hits = map(&form);
    let clicked = field_at(&hits, 10, 4);
    assert_eq!(clicked, Some(1));
    assert_eq!(form.step(clicked, true), Some(0));
    assert_eq!(form.step(Some(0), false), Some(1));
    assert_eq!(form.step(Some(0), true), Some(1));
}

#[test]
fn the_first_error_is_where_focus_goes_after_a_failed_submit() {
    let form = form();
    assert!(!form.is_valid());
    assert_eq!(form.first_error(), Some(1));
    let hits = map(&form.clone().focus(form.first_error()));
    assert_eq!(field_at(&hits, 10, 4), form.first_error());
}

#[test]
fn a_dialog_over_the_form_takes_the_click() {
    let mut hits = map(&form());
    hits.push(HitId(900), Rect::new(2, 1, 20, 3), 1);
    assert_eq!(hits.pick(5, 2), Some(HitId(900)));
    assert_eq!(field_at(&hits, 5, 2), None);
    assert_eq!(field_at(&hits, 5, 4), Some(1));
}
