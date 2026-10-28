// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::{ClusterTable, Grid, Rect, Size, Style};
use lunatui_runtime::keymap::{Command, KeyChord, Keymap};
use lunatui_term::{KeyCode, Modifiers};
use lunatui_theme::{Theme, Variant};
use lunatui_widgets::{KeyHintBar, RenderCx, Widget};

const SAVE: &[KeyChord] = &[
    KeyChord::new(KeyCode::Char('s'), Modifiers::new().control()),
    KeyChord::new(KeyCode::Char('s'), Modifiers::new()),
];
const QUIT: &[KeyChord] = &[KeyChord::new(KeyCode::Char('q'), Modifiers::new())];

fn map() -> Keymap {
    let mut keymap = Keymap::new();
    keymap.register(Command::new("save", "Save", SAVE));
    keymap.register(Command::new("quit", "Quit", QUIT));
    keymap
}

#[test]
fn draws_current_keymap_commands_in_registration_order() {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(28, 1);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);

    KeyHintBar::new(&map()).render(Rect::new(0, 0, 28, 1), &mut cx);

    assert!(
        grid.row_display(0, &clusters)
            .starts_with("[Ctrl+S] Save  [Q] Quit")
    );
    assert_eq!(
        grid.get(23, 0).map(lunatui_core::Cell::style),
        Some(Style::new())
    );
}

#[test]
fn clips_to_area_and_measures_one_row() {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(9, 2);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    KeyHintBar::new(&map()).render(Rect::new(2, 1, 6, 1), &mut cx);

    assert_eq!(grid.row_display(1, &clusters), "  [Ctrl+ ");
    assert_eq!(
        grid.get(0, 1).map(lunatui_core::Cell::style),
        Some(Style::new())
    );
    assert_eq!(
        KeyHintBar::new(&map()).measure(Size::new(20, 2)),
        Size::new(20, 1)
    );
}
