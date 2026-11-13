// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::{ClusterTable, Grid, Rect, Style};
use lunatui_runtime::{FocusDirection, FocusGroup, FocusManager, HitId};
use lunatui_term::{Event, KeyCode, MouseButton, MouseKind};
use lunatui_test::{Component, Harness};

const SLOT: u16 = 4;

struct Row {
    items: Vec<u32>,
    focus: FocusManager,
}

impl Row {
    fn new(items: &[u32]) -> Self {
        let mut row = Self {
            items: items.to_vec(),
            focus: FocusManager::new(),
        };
        row.sync();
        row
    }

    fn disabled(mut self, ids: &[u32]) -> Self {
        for id in ids {
            self.focus.set_disabled(HitId(*id), true);
        }
        self
    }

    fn group(mut self, members: &[u32]) -> Self {
        let ids: Vec<HitId> = members.iter().map(|id| HitId(*id)).collect();
        self.focus.set_group(FocusGroup(1), &ids);
        self
    }

    fn focused_on(mut self, id: u32) -> Self {
        self.focus.focus(HitId(id));
        self
    }

    fn area(&self, id: u32) -> Option<Rect> {
        let slot = self.items.iter().position(|item| *item == id)?;
        Some(Rect::new(u16::try_from(slot).ok()? * SLOT, 0, 3, 1))
    }

    fn sync(&mut self) {
        let entries = self
            .items
            .iter()
            .filter_map(|id| Some((HitId(*id), self.area(*id)?)))
            .collect();
        self.focus.sync(entries);
    }
}

impl Component for Row {
    fn update(&mut self, event: &Event) {
        match event {
            Event::Key(key) => match key.code {
                KeyCode::Tab => self.focus.tab(key.modifiers.is_shift()),
                KeyCode::Left => self.focus.arrow(FocusDirection::Left),
                KeyCode::Right => self.focus.arrow(FocusDirection::Right),
                KeyCode::Char('x') => {
                    if let Some(HitId(id)) = self.focus.focused() {
                        self.items.retain(|item| *item != id);
                        self.sync();
                    }
                }
                KeyCode::Char('d') => {
                    if let Some(id) = self.focus.focused() {
                        self.focus.set_disabled(id, true);
                    }
                }
                _ => {}
            },
            Event::Mouse(mouse) if mouse.kind == MouseKind::Down(MouseButton::Left) => {
                let hit = self.items.iter().find(|id| {
                    self.area(**id).is_some_and(|area| {
                        mouse.column >= area.x && mouse.column < area.x + area.width
                    })
                });
                if let Some(id) = hit {
                    self.focus.focus(HitId(*id));
                }
            }
            _ => {}
        }
    }

    fn render(&mut self, grid: &mut Grid, _area: Rect, clusters: &mut ClusterTable) {
        for id in &self.items {
            let Some(area) = self.area(*id) else {
                continue;
            };
            let (open, close) = if self.focus.focused() == Some(HitId(*id)) {
                ('<', '>')
            } else if self.focus.is_disabled(HitId(*id)) {
                ('(', ')')
            } else {
                ('[', ']')
            };
            grid.put_str(
                area.x,
                area.y,
                Style::default(),
                &format!("{open}{id}{close}"),
                clusters,
            );
        }
    }

    fn focus(&self) -> Option<usize> {
        self.focus.focused().map(|HitId(id)| id as usize)
    }
}

fn mount(row: Row) -> Harness<Row> {
    Harness::mount(row, 24, 1)
}

#[test]
fn tab_and_shift_tab_skip_disabled_controls() {
    let mut harness = mount(Row::new(&[1, 2, 3, 4]).disabled(&[2, 3]).focused_on(1));
    harness.assert_rows(&["<1> (2) (3) [4]"]);
    harness.key("Tab");
    harness.assert_focus(Some(4));
    harness.key("Tab");
    harness.assert_focus(Some(1));
    harness.key("S-Tab");
    harness.assert_focus(Some(4));
}

#[test]
fn tab_from_nothing_lands_on_the_first_enabled_control() {
    let mut harness = mount(Row::new(&[1, 2, 3]).disabled(&[1]));
    harness.assert_focus(Some(2));
    harness.key("Tab");
    harness.assert_focus(Some(3));
}

#[test]
fn arrows_skip_disabled_controls() {
    let mut harness = mount(Row::new(&[1, 2, 3]).disabled(&[2]).focused_on(1));
    harness.key("Right");
    harness.assert_focus(Some(3));
    harness.key("Left");
    harness.assert_focus(Some(1));
}

#[test]
fn clicking_a_disabled_control_does_not_take_focus() {
    let mut harness = mount(Row::new(&[1, 2, 3]).disabled(&[2]).focused_on(1));
    harness.click(SLOT + 1, 0);
    harness.assert_focus(Some(1));
    harness.click(2 * SLOT + 1, 0);
    harness.assert_focus(Some(3));
}

#[test]
fn removing_the_focused_control_moves_focus_to_the_previous_one() {
    let mut harness = mount(Row::new(&[1, 2, 3, 4]).focused_on(3));
    harness.key("x");
    harness.assert_focus(Some(2));
    harness.assert_rows(&["[1] <2> [4]"]);
}

#[test]
fn removing_the_first_control_moves_focus_to_the_next_one() {
    let mut harness = mount(Row::new(&[1, 2, 3]).focused_on(1));
    harness.key("x");
    harness.assert_focus(Some(2));
}

#[test]
fn recovery_skips_disabled_neighbors() {
    let mut harness = mount(Row::new(&[1, 2, 3, 4]).disabled(&[3]).focused_on(4));
    harness.key("x");
    harness.assert_focus(Some(2));
}

#[test]
fn removing_the_focused_control_keeps_focus_off_the_first_control() {
    let mut harness = mount(Row::new(&[1, 2, 3, 4, 5]).focused_on(4));
    harness.key("x");
    harness.assert_focus(Some(3));
    harness.key("x");
    harness.assert_focus(Some(2));
}

#[test]
fn disabling_the_focused_control_moves_focus_at_once() {
    let mut harness = mount(Row::new(&[1, 2, 3, 4]).focused_on(3));
    harness.key("d");
    harness.assert_focus(Some(2));
    harness.assert_rows(&["[1] <2> (3) [4]"]);
}

#[test]
fn the_only_remaining_control_keeps_focus_and_none_means_no_focus() {
    let mut harness = mount(Row::new(&[1, 2]).focused_on(2));
    harness.key("x");
    harness.assert_focus(Some(1));
    harness.key("d");
    harness.assert_focus(None);
    harness.key("Tab");
    harness.assert_focus(None);
}

#[test]
fn a_roving_group_is_one_tab_stop_and_arrows_stay_inside_it() {
    let mut harness = mount(Row::new(&[1, 2, 3, 4, 5]).group(&[2, 3, 4]).focused_on(1));
    harness.key("Tab");
    harness.assert_focus(Some(2));
    harness.keys("Right Right");
    harness.assert_focus(Some(4));
    harness.key("Right");
    harness.assert_focus(Some(4));
    harness.key("Tab");
    harness.assert_focus(Some(5));
    harness.key("S-Tab");
    harness.assert_focus(Some(4));
    harness.key("S-Tab");
    harness.assert_focus(Some(1));
}

#[test]
fn arrows_inside_a_group_skip_disabled_members() {
    let mut harness = mount(
        Row::new(&[1, 2, 3, 4])
            .group(&[1, 2, 3])
            .disabled(&[2])
            .focused_on(1),
    );
    harness.key("Right");
    harness.assert_focus(Some(3));
    harness.key("Left");
    harness.assert_focus(Some(1));
}

#[test]
fn the_group_stop_is_the_first_enabled_member_until_one_has_focus() {
    let mut harness = mount(
        Row::new(&[1, 2, 3, 4])
            .group(&[2, 3])
            .disabled(&[2])
            .focused_on(1),
    );
    harness.key("Tab");
    harness.assert_focus(Some(3));
    harness.key("Tab");
    harness.assert_focus(Some(4));
}

#[test]
fn an_arrow_from_outside_enters_the_group_and_tab_returns_to_that_member() {
    let mut harness = mount(Row::new(&[1, 2, 3, 4]).group(&[2, 3]).focused_on(1));
    harness.keys("Right Right");
    harness.assert_focus(Some(3));
    harness.key("Tab");
    harness.assert_focus(Some(4));
    harness.keys("Tab Tab");
    harness.assert_focus(Some(3));
}

#[test]
fn a_group_with_other_controls_between_its_members_keeps_one_place_in_the_tab_order() {
    let mut harness = mount(Row::new(&[1, 2, 3, 4, 5]).group(&[2, 4]).focused_on(4));
    for expected in [3, 5, 1, 4] {
        harness.key("Tab");
        harness.assert_focus(Some(expected));
    }
    for expected in [1, 5, 3, 4] {
        harness.key("S-Tab");
        harness.assert_focus(Some(expected));
    }
}

#[test]
fn forward_and_backward_tab_visit_an_interleaved_group_in_the_same_order() {
    let mut harness = mount(Row::new(&[1, 2, 3, 4, 5]).group(&[2, 4]).focused_on(1));
    for expected in [2, 3, 5] {
        harness.key("Tab");
        harness.assert_focus(Some(expected));
    }
    for expected in [3, 2, 1] {
        harness.key("S-Tab");
        harness.assert_focus(Some(expected));
    }
}
