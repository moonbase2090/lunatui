// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::{Rect, Size};
use lunatui_runtime::{HitId, HitMap};
use lunatui_widgets::{Button, ButtonState, ButtonVariant, Widget, WidgetState};

const FIRST: HitId = HitId(40);
const LABELS: [&str; 3] = ["Run checks", "Deploy", "Roll back"];

struct Toolbar {
    checked: bool,
    held: Option<usize>,
    state: ButtonState,
    clicks: Vec<usize>,
}

impl Toolbar {
    fn new() -> Self {
        Self {
            checked: false,
            held: None,
            state: ButtonState::new(),
            clicks: Vec::new(),
        }
    }

    fn button(&self, index: usize) -> Button<'static> {
        let variant = match index {
            1 => ButtonVariant::Primary,
            2 => ButtonVariant::Destructive,
            _ => ButtonVariant::Normal,
        };
        let state = if index == 1 && !self.checked {
            WidgetState::Disabled
        } else {
            WidgetState::Normal
        };
        Button::new(LABELS[index]).variant(variant).state(state)
    }

    fn slots(&self) -> Vec<Rect> {
        let mut x = 2;
        (0..LABELS.len())
            .map(|index| {
                let size = self.button(index).measure(Size::new(80, 1));
                let slot = Rect::new(x, 1, size.width, size.height);
                x += size.width + 1;
                slot
            })
            .collect()
    }

    fn map(&self) -> HitMap {
        let mut map = HitMap::new();
        map.push_indexed(
            FIRST,
            self.slots()
                .into_iter()
                .enumerate()
                .map(|(index, slot)| (index, self.button(index).hit_area(slot))),
            0,
        );
        map
    }

    fn down(&mut self, column: u16, row: u16) {
        self.held = self
            .map()
            .pick(column, row)
            .and_then(|id| id.index_in(FIRST, LABELS.len()));
        if let Some(index) = self.held {
            let hit = self.button(index).hit_area(self.slots()[index]);
            self.state.pointer_down(column, row, hit);
        }
    }

    fn up(&mut self, column: u16, row: u16) {
        if let Some(index) = self.held.take() {
            let hit = self.button(index).hit_area(self.slots()[index]);
            if self.state.pointer_up(column, row, hit) {
                self.clicks.push(index);
                if index == 0 {
                    self.checked = true;
                }
            }
        }
    }
}

#[test]
fn buttons_lay_out_at_their_measured_width() {
    assert_eq!(
        Toolbar::new().slots(),
        [
            Rect::new(2, 1, 14, 1),
            Rect::new(17, 1, 10, 1),
            Rect::new(28, 1, 13, 1)
        ]
    );
}

#[test]
fn a_disabled_button_takes_no_clicks_until_it_is_enabled() {
    let mut toolbar = Toolbar::new();
    assert_eq!(toolbar.map().pick(20, 1), None);
    toolbar.down(20, 1);
    toolbar.up(20, 1);
    assert_eq!(toolbar.clicks, Vec::<usize>::new());

    toolbar.down(3, 1);
    assert!(toolbar.state.is_pressed());
    toolbar.up(15, 1);
    assert_eq!(toolbar.clicks, [0]);
    assert_eq!(toolbar.map().pick(20, 1), Some(HitId(41)));
    toolbar.down(17, 1);
    toolbar.up(26, 1);
    assert_eq!(toolbar.clicks, [0, 1]);
}

#[test]
fn a_press_dragged_to_another_button_does_not_click_either() {
    let mut toolbar = Toolbar::new();
    toolbar.down(30, 1);
    assert_eq!(toolbar.held, Some(2));
    toolbar.up(5, 1);
    assert_eq!(toolbar.clicks, Vec::<usize>::new());
    assert!(!toolbar.state.is_pressed());
    toolbar.down(30, 1);
    toolbar.up(30, 2);
    assert_eq!(toolbar.clicks, Vec::<usize>::new());
    assert_eq!(toolbar.map().pick(16, 1), None);
}
