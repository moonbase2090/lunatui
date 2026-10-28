// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use core::time::Duration;

use lunatui_core::Rect;
use lunatui_runtime::{HitId, HitMap};
use lunatui_widgets::{Tooltip, TooltipState};

const FIRST: HitId = HitId(40);
const TIP: HitId = HitId(90);
const SCREEN: Rect = Rect::new(0, 0, 40, 10);
const BUTTONS: [(&str, Rect); 3] = [
    ("Save the open file", Rect::new(1, 0, 4, 1)),
    ("Open a file", Rect::new(6, 0, 4, 1)),
    ("Close every tab", Rect::new(36, 0, 4, 1)),
];

struct Toolbar {
    hovered: Option<usize>,
    tip: TooltipState,
}

impl Toolbar {
    fn new() -> Self {
        Self {
            hovered: None,
            tip: TooltipState::with_delay(Duration::from_millis(400)),
        }
    }

    fn map(&self, now: Duration) -> HitMap {
        let mut map = HitMap::new();
        map.push_indexed(FIRST, BUTTONS.iter().map(|(_, area)| *area).enumerate(), 0);
        if let Some(bubble) = self.shown(now).and_then(|tooltip| tooltip.area(SCREEN)) {
            map.push(TIP, bubble, 1);
        }
        map
    }

    fn shown(&self, now: Duration) -> Option<Tooltip> {
        let index = self.hovered?;
        self.tip.is_shown(now).then(|| {
            let (text, anchor) = BUTTONS[index];
            Tooltip::new(text, anchor)
        })
    }

    fn point(&mut self, column: u16, row: u16, now: Duration) {
        let over = self
            .map(now)
            .pick(column, row)
            .and_then(|id| id.index_in(FIRST, BUTTONS.len()));
        if over != self.hovered {
            self.tip.hover(false, now);
            self.hovered = over;
        }
        self.tip.hover(over.is_some(), now);
    }
}

const fn ms(millis: u64) -> Duration {
    Duration::from_millis(millis)
}

#[test]
fn resting_on_a_button_shows_its_tooltip_under_it() {
    let mut bar = Toolbar::new();
    bar.point(2, 0, ms(0));
    assert!(bar.shown(ms(399)).is_none());
    bar.point(3, 0, ms(200));
    let tip = bar.shown(ms(400)).and_then(|tooltip| tooltip.area(SCREEN));
    assert_eq!(tip, Some(Rect::new(1, 1, 20, 1)));
    assert_eq!(bar.map(ms(400)).pick(10, 1), Some(TIP));
}

#[test]
fn moving_to_another_button_starts_the_delay_again() {
    let mut bar = Toolbar::new();
    bar.point(2, 0, ms(0));
    assert!(bar.shown(ms(400)).is_some());
    bar.point(7, 0, ms(500));
    assert!(bar.shown(ms(899)).is_none());
    let tip = bar.shown(ms(900)).map(|tooltip| tooltip.anchor());
    assert_eq!(tip, Some(BUTTONS[1].1));
}

#[test]
fn the_anchor_stays_clickable_under_a_shown_tooltip() {
    for (index, (_, anchor)) in BUTTONS.iter().enumerate() {
        let mut bar = Toolbar::new();
        bar.point(anchor.x, anchor.y, ms(0));
        let map = bar.map(ms(1000));
        assert!(bar.shown(ms(1000)).is_some(), "button {index}");
        for column in anchor.x..anchor.x + anchor.width {
            assert_eq!(
                map.pick(column, anchor.y)
                    .and_then(|id| id.index_in(FIRST, 3)),
                Some(index),
                "button {index}, column {column}"
            );
        }
    }
}

#[test]
fn a_tooltip_at_the_right_edge_slides_left_and_still_misses_its_anchor() {
    let mut bar = Toolbar::new();
    bar.point(38, 0, ms(0));
    let tip = bar.shown(ms(400)).and_then(|tooltip| tooltip.area(SCREEN));
    assert_eq!(tip, Some(Rect::new(23, 1, 17, 1)));
}

#[test]
fn leaving_the_buttons_or_dismissing_hides_the_tooltip() {
    let mut bar = Toolbar::new();
    bar.point(2, 0, ms(0));
    assert!(bar.shown(ms(400)).is_some());
    bar.tip.dismiss();
    assert!(bar.shown(ms(450)).is_none());
    assert_eq!(bar.map(ms(450)).pick(10, 1), None);
    bar.point(2, 5, ms(500));
    bar.point(2, 0, ms(600));
    assert!(bar.shown(ms(999)).is_none());
    assert!(bar.shown(ms(1000)).is_some());
    bar.point(20, 5, ms(1100));
    assert!(bar.shown(ms(2000)).is_none());
}
