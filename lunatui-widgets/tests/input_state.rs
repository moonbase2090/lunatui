// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_runtime::{Handle, Outcome, SelectionMode};
use lunatui_term::{Event, KeyCode, KeyEvent, KeyKind};
use lunatui_widgets::{ListState, RadioGroup, RadioState, TabsState, WidgetState};

fn key(code: KeyCode) -> Event {
    Event::Key(KeyEvent::press(code))
}

#[test]
fn a_list_moves_with_arrows_and_pages_and_stops_at_its_ends() {
    let mut list = ListState::new(SelectionMode::Single);
    list.sync(10, 4);
    assert_eq!(list.handle(&key(KeyCode::Up)), Outcome::Changed);
    assert_eq!(
        list.focused(),
        Some(9),
        "Up from nothing lands on the last row"
    );
    assert_eq!(list.handle(&key(KeyCode::Down)), Outcome::Handled);
    assert_eq!(list.handle(&key(KeyCode::Home)), Outcome::Changed);
    assert_eq!(list.handle(&key(KeyCode::PageDown)), Outcome::Changed);
    assert_eq!(list.focused(), Some(4));
    assert_eq!(list.handle(&key(KeyCode::PageDown)), Outcome::Changed);
    assert_eq!(list.handle(&key(KeyCode::PageDown)), Outcome::Changed);
    assert_eq!(list.focused(), Some(9));
    assert_eq!(list.handle(&key(KeyCode::PageUp)), Outcome::Changed);
    assert_eq!(list.focused(), Some(5));
    assert_eq!(list.handle(&key(KeyCode::Left)), Outcome::Ignored);
}

#[test]
fn space_chooses_rows_and_enter_activates_the_focused_one() {
    let mut single = ListState::new(SelectionMode::Single);
    single.sync(3, 3);
    assert_eq!(single.handle(&key(KeyCode::Enter)), Outcome::Handled);
    single.handle(&key(KeyCode::Down));
    assert_eq!(single.handle(&key(KeyCode::Char(' '))), Outcome::Toggled(0));
    single.handle(&key(KeyCode::Down));
    single.handle(&key(KeyCode::Char(' ')));
    assert_eq!(single.selection().chosen(), &[1]);
    assert_eq!(single.handle(&key(KeyCode::Enter)), Outcome::Activated(1));

    let mut many = ListState::new(SelectionMode::Multiple);
    many.sync(3, 3);
    for _ in 0..3 {
        many.handle(&key(KeyCode::Down));
        many.handle(&key(KeyCode::Char(' ')));
    }
    assert_eq!(many.selection().chosen(), &[0, 1, 2]);
    many.sync(2, 3);
    assert_eq!(many.selection().chosen(), &[0, 1]);
    assert_eq!(many.focused(), Some(1), "focus moves off the removed row");
}

#[test]
fn key_releases_do_nothing_to_a_list() {
    let mut list = ListState::new(SelectionMode::Single);
    list.sync(3, 3);
    let release = Event::Key(KeyEvent::press(KeyCode::Down).kind(KeyKind::Release));
    assert_eq!(list.handle(&release), Outcome::Ignored);
    assert_eq!(list.focused(), None);
}

#[test]
fn tabs_move_with_left_and_right_and_wrap() {
    let mut tabs = TabsState::new(3);
    assert_eq!(tabs.selected(), 0);
    assert_eq!(tabs.handle(&key(KeyCode::Left)), Outcome::Changed);
    assert_eq!(tabs.selected(), 2);
    assert_eq!(tabs.handle(&key(KeyCode::Right)), Outcome::Changed);
    assert_eq!(tabs.selected(), 0);
    assert_eq!(tabs.handle(&key(KeyCode::End)), Outcome::Changed);
    assert_eq!(tabs.handle(&key(KeyCode::Down)), Outcome::Ignored);
    tabs.sync(2);
    assert_eq!(
        tabs.selected(),
        1,
        "the selection stays on a tab that exists"
    );
}

#[test]
fn radio_arrows_select_the_next_enabled_option_and_wrap() {
    let group = RadioGroup::new(["Low", "Medium", "High", "Max"]).disabled_options([1]);
    let mut radio = RadioState::new();
    radio.sync(&group);
    assert_eq!(radio.handle(&key(KeyCode::Down)), Outcome::Changed);
    assert_eq!(radio.selected(), Some(0));
    assert_eq!(radio.handle(&key(KeyCode::Down)), Outcome::Changed);
    assert_eq!(radio.selected(), Some(2), "Medium is disabled");
    assert_eq!(radio.handle(&key(KeyCode::Right)), Outcome::Changed);
    assert_eq!(radio.handle(&key(KeyCode::Right)), Outcome::Changed);
    assert_eq!(radio.selected(), Some(0), "wraps around");
    assert_eq!(radio.handle(&key(KeyCode::Up)), Outcome::Changed);
    assert_eq!(radio.selected(), Some(3));
    assert_eq!(radio.handle(&key(KeyCode::Char(' '))), Outcome::Handled);
    assert_eq!(radio.handle(&key(KeyCode::Char('x'))), Outcome::Ignored);
}

#[test]
fn a_disabled_radio_group_takes_no_selection() {
    let group = RadioGroup::new(["A", "B"]).state(WidgetState::Disabled);
    let mut radio = RadioState::new();
    radio.select(0);
    radio.sync(&group);
    assert_eq!(radio.selected(), None);
    assert_eq!(radio.handle(&key(KeyCode::Down)), Outcome::Handled);
    assert_eq!(radio.selected(), None);
}
