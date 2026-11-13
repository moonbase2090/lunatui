// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use std::time::Duration;

use lunatui_core::Rect;
use lunatui_runtime::{Clock, FocusManager, HitId, LayerEvent, LayerSlot, LayerStack};
use lunatui_term::{Event, KeyCode, KeyEvent, MouseButton, MouseEvent, MouseKind};
use lunatui_widgets::{
    CommandPalette, DialogKind, DialogState, PaletteEntry, PopoverState, Select, ToastKind,
    ToastScope, ToastStack,
};

const SCREEN: Rect = Rect::new(0, 0, 60, 20);
const FIELD_ID: HitId = HitId(1);
const LIST_ID: HitId = HitId(2);
const SELECT_AREA: Rect = Rect::new(2, 2, 20, 8);
const FRUIT: [&str; 4] = ["apple", "banana", "cherry", "damson"];
const ENTRIES: [PaletteEntry<'static>; 2] = [
    PaletteEntry::new("Open file"),
    PaletteEntry::new("Close tab"),
];

fn esc() -> Event {
    Event::Key(KeyEvent::press(KeyCode::Esc))
}

fn press(column: u16, row: u16) -> Event {
    Event::Mouse(MouseEvent::new(
        MouseKind::Down(MouseButton::Left),
        column,
        row,
    ))
}

fn base() -> Vec<(HitId, Rect)> {
    vec![
        (FIELD_ID, Rect::new(2, 2, 20, 1)),
        (LIST_ID, Rect::new(30, 2, 20, 10)),
    ]
}

struct Screen {
    layers: LayerStack,
    focus: FocusManager,
    select: LayerSlot,
    chosen: Option<usize>,
    delivered: Vec<(u16, u16)>,
}

impl Screen {
    fn new() -> Self {
        let mut screen = Self {
            layers: LayerStack::new(),
            focus: FocusManager::new(),
            select: LayerSlot::new(),
            chosen: None,
            delivered: Vec::new(),
        };
        screen.frame();
        screen
    }

    fn widget(&self) -> Select<'static> {
        Select::new(&FRUIT, self.chosen).open(self.select.is_open())
    }

    fn frame(&mut self) {
        let list = self.widget().list_area(SELECT_AREA);
        self.select.place(&mut self.layers, list);
        self.focus.sync(self.layers.focus_entries(base()));
    }

    fn event(&mut self, event: &Event) {
        let deliver = match self.layers.handle(event, &mut self.focus) {
            LayerEvent::Pass => true,
            LayerEvent::Consumed => false,
            LayerEvent::Dismissed { deliver, .. } => {
                self.select.sync(&self.layers);
                deliver
            }
        };
        if deliver && let Event::Mouse(mouse) = event {
            let (column, row) = (mouse.column, mouse.row);
            let select = self.widget();
            if row == SELECT_AREA.y && (2..22).contains(&column) {
                if self.select.is_open() {
                    self.select.close(&mut self.layers, &mut self.focus);
                } else {
                    self.select
                        .open(&mut self.layers, select.layer(SELECT_AREA, Some(FIELD_ID)));
                }
            } else if let Some(index) = select.option_at(SELECT_AREA, column, row) {
                self.chosen = Some(index);
                self.select.close(&mut self.layers, &mut self.focus);
            } else {
                self.delivered.push((column, row));
            }
        }
        self.frame();
    }
}

#[test]
fn the_select_list_opens_from_its_field_and_a_second_press_there_closes_it() {
    let mut screen = Screen::new();
    screen.event(&press(5, 2));
    assert!(screen.select.is_open());
    assert_eq!(screen.layers.len(), 1);
    screen.event(&press(6, 2));
    assert!(!screen.select.is_open());
    assert!(screen.layers.is_empty());
}

#[test]
fn choosing_an_option_closes_the_list_and_focus_returns_to_the_field() {
    let mut screen = Screen::new();
    screen.focus.focus(LIST_ID);
    screen.event(&press(5, 2));
    screen.event(&press(5, 5));
    assert_eq!(screen.chosen, Some(2));
    assert!(!screen.select.is_open());
    assert_eq!(screen.focus.focused(), Some(FIELD_ID));
}

#[test]
fn esc_or_a_press_outside_closes_the_select_list() {
    let mut screen = Screen::new();
    screen.event(&press(5, 2));
    screen.event(&esc());
    assert!(!screen.select.is_open());
    assert_eq!(screen.chosen, None);

    screen.event(&press(5, 2));
    screen.event(&press(40, 15));
    assert!(!screen.select.is_open());
    assert_eq!(
        screen.delivered,
        vec![(40, 15)],
        "the press that closed the list still lands"
    );
}

#[test]
fn the_command_palette_is_modal_esc_closes_it_and_presses_outside_are_dropped() {
    let mut screen = Screen::new();
    screen.focus.focus(LIST_ID);
    let palette = CommandPalette::new("", 0, &ENTRIES);
    let mut slot = LayerSlot::new();
    slot.open(
        &mut screen.layers,
        palette.layer(SCREEN, screen.focus.focused()),
    );
    screen.frame();
    assert_eq!(screen.focus.focused(), None, "Tab cannot reach the screen");
    screen.event(&press(5, 2));
    assert!(slot.is_open());
    assert!(
        !screen.select.is_open(),
        "the press never reached the select"
    );
    assert_eq!(screen.delivered, Vec::new());

    screen.event(&esc());
    assert!(slot.sync(&screen.layers));
    assert_eq!(screen.focus.focused(), Some(LIST_ID));
    screen.event(&press(5, 2));
    assert!(screen.select.is_open(), "the screen takes presses again");
}

#[test]
fn toasts_sit_in_a_layer_that_leaves_esc_and_outside_presses_alone_until_they_expire() {
    let mut screen = Screen::new();
    let mut clock = Clock::new();
    let mut toasts = ToastStack::new();
    let mut slot = LayerSlot::new();
    let ttl = Duration::from_secs(4);
    toasts.push(
        clock.now(),
        Some(ttl),
        ToastScope::Status,
        ToastKind::Success,
        "Saved",
    );
    let mut sync_toasts = |screen: &mut Screen, slot: &mut LayerSlot, now: Duration| match toasts
        .layer(now, SCREEN, |_| None)
    {
        Some(layer) => {
            slot.open(&mut screen.layers, layer);
            slot.place(&mut screen.layers, layer.area());
        }
        None => {
            slot.close(&mut screen.layers, &mut screen.focus);
        }
    };
    sync_toasts(&mut screen, &mut slot, clock.now());
    assert!(slot.is_open());

    screen.event(&press(5, 2));
    assert!(screen.select.is_open(), "the select opens under the toast");
    screen.event(&esc());
    assert!(
        !screen.select.is_open(),
        "Esc closes the select, not the toast"
    );
    assert!(slot.is_open());
    screen.event(&press(40, 15));
    assert!(slot.is_open());
    assert_eq!(screen.delivered, vec![(40, 15)]);

    clock.advance(ttl - Duration::from_millis(1));
    sync_toasts(&mut screen, &mut slot, clock.now());
    assert!(slot.is_open());
    clock.advance(Duration::from_millis(1));
    sync_toasts(&mut screen, &mut slot, clock.now());
    assert!(!slot.is_open());
    assert!(screen.layers.is_empty());
}

#[test]
fn a_dialog_over_an_open_popover_closes_first_then_the_popover() {
    let mut screen = Screen::new();
    let anchor = Rect::new(30, 2, 6, 1);
    let mut popover = PopoverState::new();
    let mut dialog = DialogState::new();
    popover.open(&mut screen.layers, anchor, Some(LIST_ID));
    popover.place(&mut screen.layers, Some(Rect::new(30, 3, 12, 4)));
    dialog.open(DialogKind::Confirm, &mut screen.layers, Some(FIELD_ID));
    dialog.place(&mut screen.layers, Rect::new(10, 6, 40, 8));
    assert_eq!(screen.layers.len(), 2);

    screen.event(&press(31, 4));
    assert!(popover.is_open(), "the dialog blocks the popover under it");
    assert_eq!(dialog.sync(&screen.layers), None);

    screen.event(&esc());
    let closed = dialog.sync(&screen.layers);
    assert!(closed.is_some(), "Esc cancels the dialog first");
    assert!(popover.is_open());
    assert!(!popover.sync(&screen.layers));
    assert_eq!(screen.focus.focused(), Some(FIELD_ID));

    screen.event(&esc());
    assert!(popover.sync(&screen.layers));
    assert!(screen.layers.is_empty());
    assert_eq!(screen.focus.focused(), Some(LIST_ID));
}
