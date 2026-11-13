// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::{Rect, Size};
use lunatui_runtime::{FocusManager, HitId, HitMap, LayerEvent, LayerStack};
use lunatui_term::{Event, KeyCode, KeyEvent, MouseButton, MouseEvent, MouseKind};
use lunatui_widgets::{Paragraph, Popover, PopoverState};

const VIEW: Rect = Rect::new(30, 0, 6, 1);
const SCREEN: Rect = Rect::new(0, 0, 40, 12);
const BUTTON: HitId = HitId(10);
const FIRST_ROW: HitId = HitId(20);
const FIRST_OPTION: HitId = HitId(50);
const OPTIONS: [&str; 3] = ["Compact", "Comfortable", "Spacious"];

fn popover() -> Popover<Paragraph> {
    Popover::new(Paragraph::new(OPTIONS.join("\n")), VIEW)
        .title("Density")
        .size(Size::new(12, 3))
}

struct Screen {
    layers: LayerStack,
    focus: FocusManager,
    view: PopoverState,
    density: usize,
    pressed: Vec<HitId>,
}

impl Screen {
    fn new(density: usize) -> Self {
        let mut screen = Self {
            layers: LayerStack::new(),
            focus: FocusManager::new(),
            view: PopoverState::new(),
            density,
            pressed: Vec::new(),
        };
        screen.frame();
        screen
    }

    fn frame(&mut self) {
        let panel = self.panel();
        self.view.place(&mut self.layers, panel);
        self.focus.sync(
            self.layers
                .focus_entries(vec![(BUTTON, VIEW), (FIRST_ROW, Rect::new(0, 2, 40, 1))]),
        );
    }

    fn map(&self) -> HitMap {
        let mut map = HitMap::new();
        map.push(BUTTON, VIEW, 0);
        map.push_indexed(
            FIRST_ROW,
            (0..10_u16).map(|row| (usize::from(row), Rect::new(0, 2 + row, 40, 1))),
            0,
        );
        if let (Some(panel), Some(z)) = (
            self.panel(),
            self.view.layer().and_then(|id| self.layers.z(id)),
        ) {
            map.push(HitId(90), panel, z);
            map.push_indexed(
                FIRST_OPTION,
                (0..3_u16).map(|row| {
                    (
                        usize::from(row),
                        Rect::new(panel.x + 1, panel.y + 1 + row, 12, 1),
                    )
                }),
                z + 1,
            );
        }
        map
    }

    fn panel(&self) -> Option<Rect> {
        self.view
            .is_open()
            .then(|| popover().area(SCREEN))
            .flatten()
    }

    fn event(&mut self, event: &Event) {
        let deliver = match self.layers.handle(event, &mut self.focus) {
            LayerEvent::Pass => true,
            LayerEvent::Consumed => false,
            LayerEvent::Dismissed { deliver, .. } => {
                self.view.sync(&self.layers);
                deliver
            }
        };
        if deliver && let Event::Mouse(mouse) = event {
            let hit = self.map().pick(mouse.column, mouse.row);
            self.pressed.extend(hit);
            if hit == Some(BUTTON) {
                self.view
                    .toggle(&mut self.layers, &mut self.focus, VIEW, Some(BUTTON));
            } else if let Some(option) = hit.and_then(|id| id.index_in(FIRST_OPTION, 3)) {
                self.density = option;
                self.view.close(&mut self.layers, &mut self.focus);
            }
        }
        self.frame();
    }

    fn click(&mut self, column: u16, row: u16) -> Option<HitId> {
        self.event(&Event::Mouse(MouseEvent::new(
            MouseKind::Down(MouseButton::Left),
            column,
            row,
        )));
        self.pressed.last().copied()
    }
}

#[test]
fn the_button_opens_the_popover_under_it_slid_left_to_fit() {
    let mut screen = Screen::new(0);
    assert_eq!(screen.click(32, 0), Some(BUTTON));
    assert_eq!(screen.panel(), Some(Rect::new(26, 1, 14, 5)));
    assert_eq!(screen.layers.len(), 1);
}

#[test]
fn the_open_panel_takes_clicks_over_the_rows_under_it() {
    let mut screen = Screen::new(0);
    screen.click(32, 0);
    assert_eq!(screen.map().pick(30, 3), Some(HitId(51)));
    assert_eq!(screen.map().pick(10, 3), Some(HitId(21)));
    assert_eq!(screen.click(28, 4), Some(HitId(52)));
    assert_eq!(screen.density, 2);
    assert!(!screen.view.is_open(), "choosing an option closes it");
    assert!(screen.layers.is_empty());
}

#[test]
fn a_click_elsewhere_closes_it_and_still_reaches_what_was_clicked() {
    let mut screen = Screen::new(1);
    screen.click(31, 0);
    assert!(screen.view.is_open());
    assert_eq!(screen.click(5, 6), Some(HitId(24)));
    assert!(!screen.view.is_open());
    assert_eq!(screen.density, 1);
    assert_eq!(
        screen.map().pick(30, 3),
        Some(HitId(21)),
        "rows clickable again"
    );
}

#[test]
fn the_button_closes_an_open_popover_and_esc_does_too_and_focus_returns_to_it() {
    let mut screen = Screen::new(0);
    screen.click(33, 0);
    assert_eq!(screen.click(34, 0), Some(BUTTON));
    assert!(!screen.view.is_open());
    screen.click(33, 0);
    screen.focus.focus(FIRST_ROW);
    screen.event(&Event::Key(KeyEvent::press(KeyCode::Esc)));
    assert_eq!(screen.panel(), None);
    assert!(screen.layers.is_empty());
    assert_eq!(screen.focus.focused(), Some(BUTTON));
}
