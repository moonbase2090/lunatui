// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::{Rect, Size};
use lunatui_runtime::{HitId, HitMap};
use lunatui_widgets::{Paragraph, Popover, PopoverClick, PopoverState};

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
    view: PopoverState,
    density: usize,
}

impl Screen {
    fn map(&self) -> HitMap {
        let mut map = HitMap::new();
        map.push(BUTTON, VIEW, 0);
        map.push_indexed(
            FIRST_ROW,
            (0..10_u16).map(|row| (usize::from(row), Rect::new(0, 2 + row, 40, 1))),
            0,
        );
        if let Some(panel) = self.panel() {
            map.push(HitId(90), panel, 1);
            map.push_indexed(
                FIRST_OPTION,
                (0..3_u16).map(|row| {
                    (
                        usize::from(row),
                        Rect::new(panel.x + 1, panel.y + 1 + row, 12, 1),
                    )
                }),
                2,
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

    fn click(&mut self, column: u16, row: u16) -> (PopoverClick, Option<HitId>) {
        let hit = self.map().pick(column, row);
        let panel = self.panel();
        let outcome = self.view.click(column, row, VIEW, panel);
        if outcome == PopoverClick::Inside
            && let Some(option) = hit.and_then(|id| id.index_in(FIRST_OPTION, 3))
        {
            self.density = option;
            self.view.close();
        }
        (outcome, hit)
    }
}

#[test]
fn the_button_opens_the_popover_under_it_slid_left_to_fit() {
    let mut screen = Screen {
        view: PopoverState::new(),
        density: 0,
    };
    assert_eq!(screen.click(32, 0), (PopoverClick::Anchor, Some(BUTTON)));
    assert_eq!(screen.panel(), Some(Rect::new(26, 1, 14, 5)));
}

#[test]
fn the_open_panel_takes_clicks_over_the_rows_under_it() {
    let mut screen = Screen {
        view: PopoverState::new(),
        density: 0,
    };
    screen.click(32, 0);
    assert_eq!(screen.map().pick(30, 3), Some(HitId(51)));
    assert_eq!(screen.map().pick(10, 3), Some(HitId(21)));
    assert_eq!(screen.click(28, 4), (PopoverClick::Inside, Some(HitId(52))));
    assert_eq!(screen.density, 2);
    assert!(!screen.view.is_open(), "choosing an option closes it");
}

#[test]
fn a_click_elsewhere_closes_it_and_still_reaches_what_was_clicked() {
    let mut screen = Screen {
        view: PopoverState::new(),
        density: 1,
    };
    screen.click(31, 0);
    assert!(screen.view.is_open());
    assert_eq!(screen.click(5, 6), (PopoverClick::Outside, Some(HitId(24))));
    assert!(!screen.view.is_open());
    assert_eq!(screen.density, 1);
    assert_eq!(
        screen.map().pick(30, 3),
        Some(HitId(21)),
        "rows clickable again"
    );
}

#[test]
fn the_button_closes_an_open_popover_and_escape_does_too() {
    let mut screen = Screen {
        view: PopoverState::new(),
        density: 0,
    };
    screen.click(33, 0);
    assert_eq!(screen.click(34, 0), (PopoverClick::Anchor, Some(BUTTON)));
    assert!(!screen.view.is_open());
    screen.click(33, 0);
    screen.view.close();
    assert_eq!(screen.panel(), None);
}
