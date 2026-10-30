// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::{Cell, ClusterTable, Grid, Rect, Style, UnderlineStyle};
use lunatui_theme::{Theme, Variant};
use lunatui_widgets::{Menu, MenuEvent, MenuItem, MenuState, RenderCx, Widget};

fn menus() -> MenuState<&'static str> {
    MenuState::new(vec![
        MenuItem::group(
            "File",
            vec![
                MenuItem::item("New", "new").mnemonic('n'),
                MenuItem::item("Open", "open").mnemonic('o'),
                MenuItem::group(
                    "Recent",
                    vec![MenuItem::item("notes.md", "notes").mnemonic('n')],
                )
                .mnemonic('r'),
                MenuItem::item("Print", "print")
                    .mnemonic('p')
                    .disabled(true),
            ],
        )
        .mnemonic('f'),
        MenuItem::group(
            "Edit",
            vec![
                MenuItem::item("Copy", "copy").mnemonic('c'),
                MenuItem::item("Cut", "cut").mnemonic('c'),
                MenuItem::item("Paste", "paste").mnemonic('p'),
            ],
        )
        .mnemonic('e'),
        MenuItem::group("View", vec![MenuItem::item("Zoom", "zoom")])
            .mnemonic('v')
            .disabled(true),
        MenuItem::item("Help", "help").mnemonic('h'),
    ])
}

fn events(state: &mut MenuState<&'static str>) -> Vec<MenuEvent<&'static str>> {
    state.drain_events().collect()
}

fn draw(menu: &Menu<'_, &'static str>, width: u16, height: u16) -> Grid {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(width, height);
    let mut clusters = ClusterTable::new();
    menu.render(
        Rect::new(0, 0, width, height),
        &mut RenderCx::new(&mut grid, &mut clusters, &theme),
    );
    grid
}

fn underlined(grid: &Grid, x: u16, y: u16) -> bool {
    grid.get(x, y)
        .map(Cell::style)
        .unwrap_or_default()
        .attrs
        .underline()
        == UnderlineStyle::Single
}

fn underlined_cells(grid: &Grid, y: u16) -> Vec<u16> {
    (0..grid.width())
        .filter(|x| underlined(grid, *x, y))
        .collect()
}

#[test]
fn a_top_level_mnemonic_opens_its_menu_at_the_first_enabled_item() {
    let mut state = menus();
    assert!(state.press_mnemonic('e'));
    assert_eq!(state.selected_path(), [1, 0]);
    assert_eq!(events(&mut state), []);

    let mut state = menus();
    assert!(state.press_mnemonic('F'));
    assert_eq!(state.selected_path(), [0, 0]);

    let mut state = menus();
    state.activate();
    assert_eq!(state.selected_path(), [0]);
    assert!(state.press_mnemonic('e'));
    assert_eq!(state.selected_path(), [1, 0]);
}

#[test]
fn a_mnemonic_in_an_open_menu_chooses_the_item() {
    let mut state = menus();
    state.press_mnemonic('e');
    assert!(state.press_mnemonic('p'));
    assert_eq!(state.selected_path(), [1, 2]);
    assert_eq!(events(&mut state), [MenuEvent::Selected("paste")]);

    let mut state = menus();
    state.press_mnemonic('f');
    assert!(state.press_mnemonic('r'));
    assert_eq!(state.selected_path(), [0, 2, 0]);
    assert!(state.press_mnemonic('N'));
    assert_eq!(events(&mut state), [MenuEvent::Selected("notes")]);
}

#[test]
fn a_shared_mnemonic_moves_between_its_items_without_choosing() {
    let mut state = menus();
    state.press_mnemonic('e');
    assert_eq!(state.selected_path(), [1, 0]);
    assert!(state.press_mnemonic('c'));
    assert_eq!(state.selected_path(), [1, 1]);
    assert!(state.press_mnemonic('c'));
    assert_eq!(state.selected_path(), [1, 0]);
    assert_eq!(events(&mut state), []);
}

#[test]
fn unknown_and_disabled_mnemonics_change_nothing() {
    let mut state = menus();
    assert!(!state.press_mnemonic('v'));
    assert!(!state.press_mnemonic('x'));
    assert!(!state.is_active());

    state.press_mnemonic('f');
    assert!(!state.press_mnemonic('p'));
    assert!(!state.press_mnemonic('e'));
    assert_eq!(state.selected_path(), [0, 0]);
    assert_eq!(events(&mut state), []);

    let mut help = menus();
    assert!(help.press_mnemonic('h'));
    assert_eq!(help.selected_path(), [3]);
    assert_eq!(events(&mut help), [MenuEvent::Selected("help")]);
    assert_eq!(
        MenuItem::item("Go", 1).mnemonic('g').mnemonic_key(),
        Some('g')
    );
    assert_eq!(MenuItem::item("Go", 1).mnemonic_key(), None);
}

#[test]
fn open_highlights_an_enabled_top_level_item() {
    let mut state = menus();
    state.press_mnemonic('f');
    assert!(state.open(1));
    assert_eq!(state.selected_path(), [1]);
    assert!(!state.open(2));
    assert!(!state.open(9));
    assert_eq!(state.selected_path(), [1]);
}

#[test]
fn mnemonic_letters_are_underlined_in_the_bar_and_open_menu() {
    let mut state = menus();
    let grid = draw(&Menu::new(&state), 40, 8);
    assert_eq!(underlined_cells(&grid, 0), [2, 8, 20]);

    state.press_mnemonic('f');
    let grid = draw(&Menu::new(&state), 40, 8);
    assert_eq!(underlined_cells(&grid, 2), [3]);
    assert_eq!(underlined_cells(&grid, 3), [3]);
    assert_eq!(underlined_cells(&grid, 4), [3]);
    assert_eq!(underlined_cells(&grid, 5), Vec::<u16>::new());

    let plain = draw(&Menu::new(&state).mnemonics(false), 40, 8);
    for y in 0..8 {
        assert_eq!(underlined_cells(&plain, y), Vec::<u16>::new(), "row {y}");
    }
}

#[test]
fn a_mnemonic_later_in_the_label_and_a_clipped_one() {
    let state = MenuState::new(vec![
        MenuItem::item("Export", "export").mnemonic('x'),
        MenuItem::item("Quit", "quit").mnemonic('t'),
    ]);
    let grid = draw(&Menu::new(&state), 20, 1);
    assert_eq!(underlined_cells(&grid, 0), [3, 13]);
    let clipped = draw(&Menu::new(&state), 12, 1);
    assert_eq!(underlined_cells(&clipped, 0), [3]);
}

#[test]
fn bar_item_at_finds_the_top_level_item_under_a_click() {
    let state = menus();
    let menu = Menu::new(&state);
    let area = Rect::new(2, 1, 40, 8);
    assert_eq!(menu.bar_item_at(area, 2, 1), None);
    assert_eq!(menu.bar_item_at(area, 1, 1), None);
    let far = Rect::new(30, 0, 40, 1);
    assert_eq!(menu.bar_item_at(far, 32, 0), Some(0));
    assert_eq!(menu.bar_item_at(far, 69, 0), None);
    assert_eq!(menu.bar_item_at(far, 70, 0), None);
    assert_eq!(menu.bar_item_at(area, 3, 1), Some(0));
    assert_eq!(menu.bar_item_at(area, 8, 1), Some(0));
    assert_eq!(menu.bar_item_at(area, 9, 1), Some(1));
    assert_eq!(menu.bar_item_at(area, 26, 1), Some(3));
    assert_eq!(menu.bar_item_at(area, 27, 1), None);
    assert_eq!(menu.bar_item_at(area, 5, 2), None);
    assert_eq!(menu.bar_item_at(area, 42, 1), None);
    assert_eq!(menu.bar_item_at(Rect::new(2, 1, 4, 1), 7, 1), None);
}

#[test]
fn styled_cells_keep_the_rest_of_their_style() {
    let state = menus();
    let grid = draw(&Menu::new(&state), 40, 1);
    let under = grid.get(2, 0).map(Cell::style).unwrap_or_default();
    let next = grid.get(3, 0).map(Cell::style).unwrap_or_default();
    assert_eq!(
        (under.fg, under.bg),
        (next.fg, next.bg),
        "only the underline differs"
    );
    assert_eq!(
        Style::new().with_attrs(under.attrs.set_underline(UnderlineStyle::None)),
        Style::new().with_attrs(next.attrs)
    );
}

#[test]
fn letters_reach_the_dropdown_of_an_opened_group_then_the_bar() {
    let mut state = menus();
    assert!(state.open(1));
    assert!(state.press_mnemonic('p'));
    assert_eq!(state.selected_path(), [1, 2]);
    assert_eq!(events(&mut state), [MenuEvent::Selected("paste")]);

    let mut state = menus();
    state.open(0);
    assert!(state.press_mnemonic('e'));
    assert_eq!(state.selected_path(), [1, 0]);

    let mut state = menus();
    state.open(1);
    assert!(state.press_mnemonic('c'));
    assert_eq!(state.selected_path(), [1, 0]);
    assert_eq!(events(&mut state), []);

    let mut state = menus();
    state.open(0);
    assert!(!state.press_mnemonic('x'));
    assert_eq!(state.selected_path(), [0]);

    let mut leaf = menus();
    leaf.open(3);
    assert!(leaf.press_mnemonic('f'));
    assert_eq!(leaf.selected_path(), [0, 0]);
}

#[test]
fn a_highlighted_group_in_a_dropdown_does_not_take_its_childrens_keys() {
    let mut state = menus();
    state.press_mnemonic('f');
    state.down();
    state.down();
    assert_eq!(state.selected_path(), [0, 2]);
    assert!(state.press_mnemonic('n'));
    assert_eq!(state.selected_path(), [0, 0]);
    assert_eq!(events(&mut state), [MenuEvent::Selected("new")]);
}

#[test]
fn a_shared_bar_key_moves_on_from_a_highlighted_leaf() {
    let mut state = MenuState::new(vec![
        MenuItem::item("Sync", "sync").mnemonic('s'),
        MenuItem::item("Help", "help").mnemonic('h'),
        MenuItem::item("Share", "share").mnemonic('s'),
    ]);
    assert!(state.open(1));
    assert!(state.press_mnemonic('s'));
    assert_eq!(state.selected_path(), [2]);
    assert!(state.press_mnemonic('s'));
    assert_eq!(state.selected_path(), [0]);
    assert_eq!(events(&mut state), []);
}
