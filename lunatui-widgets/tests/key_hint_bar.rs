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

fn row(keymap: &Keymap, width: u16, gap: Option<u16>) -> String {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(width, 1);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    let bar = match gap {
        Some(cells) => KeyHintBar::new(keymap).gap(cells),
        None => KeyHintBar::new(keymap),
    };
    bar.render(Rect::new(0, 0, width, 1), &mut cx);
    grid.row_display(0, &clusters)
}

#[test]
fn the_gap_between_hints_is_configurable_and_never_trails() {
    assert_eq!(row(&map(), 30, None), "[Ctrl+S] Save  [Q] Quit       ");
    assert_eq!(row(&map(), 30, Some(0)), "[Ctrl+S] Save[Q] Quit         ");
    assert_eq!(row(&map(), 30, Some(5)), "[Ctrl+S] Save     [Q] Quit    ");
}

#[test]
fn a_gap_is_cut_by_the_area_edge() {
    assert_eq!(row(&map(), 14, Some(5)), "[Ctrl+S] Save ");
    assert_eq!(row(&map(), 15, Some(5)), "[Ctrl+S] Save  ");
    assert_eq!(row(&map(), 16, Some(5)), "[Ctrl+S] Save   ");
}

#[test]
fn clipping_ends_at_each_part_of_a_hint() {
    assert_eq!(row(&map(), 4, None), "[Ctr");
    assert_eq!(row(&map(), 8, None), "[Ctrl+S]");
    assert_eq!(row(&map(), 9, None), "[Ctrl+S] ");
    assert_eq!(row(&map(), 10, None), "[Ctrl+S] S");
    assert_eq!(row(&map(), 13, None), "[Ctrl+S] Save");
    assert_eq!(row(&map(), 1, None), "[");
}

#[test]
fn commands_without_chords_are_left_out_and_the_first_chord_is_shown() {
    let mut keymap = Keymap::new();
    keymap.register(Command::new("hidden", "Hidden", &[]));
    keymap.register(Command::new("save", "Save", SAVE));
    keymap.register(Command::new("also", "Also hidden", &[]));
    assert_eq!(row(&keymap, 20, None), "[Ctrl+S] Save       ");
    keymap.register(Command::new("quit", "Quit", QUIT));
    assert_eq!(row(&keymap, 24, None), "[Ctrl+S] Save  [Q] Quit ");
}

#[test]
fn an_empty_keymap_draws_nothing() {
    assert_eq!(row(&Keymap::new(), 6, None), "      ");
}

#[test]
fn keys_use_the_accent_and_titles_the_primary_text_color() {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(24, 1);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    KeyHintBar::new(&map()).render(Rect::new(0, 0, 24, 1), &mut cx);
    let fg = |x| {
        grid.get(x, 0)
            .map(lunatui_core::Cell::style)
            .unwrap_or_default()
            .fg
    };
    let accent = theme.resolve(lunatui_theme::Token::Accent);
    let primary = theme.resolve(lunatui_theme::Token::TextPrimary);
    assert_eq!(fg(0), accent);
    assert_eq!(fg(7), accent);
    assert_eq!(fg(8), primary);
    assert_eq!(fg(9), primary);
    assert_eq!(fg(12), primary);
    assert_eq!(fg(13), primary);
    assert_eq!(fg(14), primary);
    assert_eq!(fg(15), accent);
    assert_eq!(fg(17), accent);
    assert_eq!(fg(18), primary);
    assert_eq!(fg(19), primary);
    assert_eq!(fg(22), primary);
}

#[test]
fn empty_areas_draw_nothing_and_measure_zero() {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(10, 2);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    let keymap = map();
    let bar = KeyHintBar::new(&keymap);
    bar.render(Rect::new(0, 0, 0, 1), &mut cx);
    bar.render(Rect::new(0, 0, 10, 0), &mut cx);
    assert_eq!(grid.row_display(0, &clusters).trim(), "");
    assert_eq!(bar.measure(Size::new(0, 3)), Size::new(0, 0));
    assert_eq!(bar.measure(Size::new(7, 0)), Size::new(7, 0));
    assert_eq!(bar.measure(Size::new(7, 1)), Size::new(7, 1));
}

#[test]
fn drawing_stays_inside_an_offset_area() {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(20, 3);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    KeyHintBar::new(&map()).render(Rect::new(3, 1, 9, 2), &mut cx);
    assert_eq!(grid.row_display(0, &clusters).trim(), "");
    assert_eq!(grid.row_display(1, &clusters), "   [Ctrl+S]         ");
    assert_eq!(grid.row_display(2, &clusters).trim(), "");
}

#[test]
fn nothing_is_drawn_past_the_right_edge_of_the_area() {
    let theme = Theme::builtin(Variant::Dark);
    let full = "[Ctrl+S] Save       [Q] Quit";
    for area_width in [12_u16, 14, 15] {
        let mut grid = Grid::new(20, 1);
        let mut clusters = ClusterTable::new();
        let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
        KeyHintBar::new(&map())
            .gap(7)
            .render(Rect::new(0, 0, area_width, 1), &mut cx);
        let drawn = grid.row_display(0, &clusters);
        assert!(
            drawn.starts_with(&full[..usize::from(area_width)]),
            "{drawn:?}"
        );
        for x in area_width..20 {
            assert_eq!(
                grid.get(x, 0).map(lunatui_core::Cell::style),
                Some(Style::new()),
                "area {area_width}, column {x}"
            );
        }
    }
}
