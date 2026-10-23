// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::{ClusterTable, Grid, Rect};
use lunatui_runtime::keymap::{Command, KeyChord, Keymap, render_overlay, render_palette};
use lunatui_term::{Event, KeyCode, KeyEvent, KeyKind, Modifiers};

const OVERLAY: &str = "overlay.toggle";
const PALETTE: &str = "palette.open";
const QUIT: &str = "app.quit";

const OVERLAY_CHORDS: &[KeyChord] = &[
    KeyChord::new(KeyCode::Char('/'), Modifiers::new().control().shift()),
    KeyChord::new(KeyCode::F(1), Modifiers::new()),
];
const PALETTE_CHORDS: &[KeyChord] = &[KeyChord::new(
    KeyCode::Char('p'),
    Modifiers::new().control().shift(),
)];
const QUIT_CHORDS: &[KeyChord] = &[KeyChord::new(KeyCode::Char('q'), Modifiers::new())];

fn registry() -> Keymap {
    let mut keymap = Keymap::new();
    keymap.register(Command::new(OVERLAY, "Show shortcuts", OVERLAY_CHORDS));
    keymap.register(Command::new(
        PALETTE,
        "Open command palette",
        PALETTE_CHORDS,
    ));
    keymap.register(Command::new(QUIT, "Quit", QUIT_CHORDS));
    keymap
}

fn key(code: KeyCode, modifiers: Modifiers) -> Event {
    Event::Key(KeyEvent::press(code).modifiers(modifiers))
}

fn grid_text(grid: &Grid, clusters: &ClusterTable) -> String {
    grid.display_text(clusters)
}

#[test]
fn kitty_chord_dispatches_overlay_command() {
    let keymap = registry();
    let event = key(KeyCode::Char('/'), Modifiers::new().control().shift());
    assert_eq!(keymap.dispatch(&event), Some(OVERLAY));
}

#[test]
fn legacy_fallback_chord_dispatches_same_command() {
    let keymap = registry();
    let event = key(KeyCode::F(1), Modifiers::new());
    assert_eq!(keymap.dispatch(&event), Some(OVERLAY));
}

#[test]
fn plain_slash_does_not_dispatch_overlay() {
    let keymap = registry();
    let event = key(KeyCode::Char('/'), Modifiers::new());
    assert_eq!(keymap.dispatch(&event), None);
}

#[test]
fn modifiers_must_match_exactly() {
    let keymap = registry();
    let extra_alt = key(KeyCode::Char('/'), Modifiers::new().control().shift().alt());
    assert_eq!(keymap.dispatch(&extra_alt), None);
}

#[test]
fn release_events_do_not_dispatch() {
    let keymap = registry();
    let event = Event::Key(KeyEvent::press(KeyCode::Char('q')).kind(KeyKind::Release));
    assert_eq!(keymap.dispatch(&event), None);
}

#[test]
fn non_key_events_do_not_dispatch() {
    let keymap = registry();
    assert_eq!(
        keymap.dispatch(&Event::Focus(lunatui_term::Focus::In)),
        None
    );
}

#[test]
fn palette_filters_by_title_case_insensitively() {
    let keymap = registry();
    let ids: Vec<&str> = keymap.palette("QUIT").iter().map(|c| c.id()).collect();
    assert_eq!(ids, vec![QUIT]);
}

#[test]
fn empty_palette_query_lists_every_command_in_registration_order() {
    let keymap = registry();
    let ids: Vec<&str> = keymap.palette("").iter().map(|c| c.id()).collect();
    assert_eq!(ids, vec![OVERLAY, PALETTE, QUIT]);
}

#[test]
fn palette_entries_dispatch_to_their_own_id() {
    let keymap = registry();
    for command in keymap.commands() {
        let chord = command.chords()[0];
        let event = key(chord.code(), chord.modifiers());
        assert_eq!(keymap.dispatch(&event), Some(command.id()));
    }
}

#[test]
fn overlay_lists_every_command_with_its_chords() {
    let keymap = registry();
    let mut grid = Grid::new(60, 12);
    let mut clusters = ClusterTable::new();
    render_overlay(&keymap, &mut grid, Rect::new(0, 0, 60, 12), &mut clusters);
    let text = grid_text(&grid, &clusters);
    assert!(text.contains("Show shortcuts"), "{text}");
    assert!(text.contains("Open command palette"), "{text}");
    assert!(text.contains("Quit"), "{text}");
    assert!(text.contains("F1"), "{text}");
    assert!(text.contains("Ctrl+Shift+P"), "{text}");
}

#[test]
fn palette_renders_filtered_titles_and_marks_selection() {
    let keymap = registry();
    let mut grid = Grid::new(40, 8);
    let mut clusters = ClusterTable::new();
    render_palette(
        &keymap,
        "pal",
        0,
        &mut grid,
        Rect::new(0, 0, 40, 8),
        &mut clusters,
    );
    let text = grid_text(&grid, &clusters);
    assert!(text.contains("> Open command palette"), "{text}");
    assert!(!text.contains("Quit"), "{text}");
}

fn rows(grid: &Grid, clusters: &ClusterTable, count: u16) -> Vec<String> {
    (0..count)
        .map(|row| grid.row_display(row, clusters).trim_end().to_string())
        .collect()
}

#[test]
fn overlay_snapshot_matches_expected_rows() {
    let keymap = registry();
    let mut grid = Grid::new(60, 6);
    let mut clusters = ClusterTable::new();
    render_overlay(&keymap, &mut grid, Rect::new(0, 0, 60, 6), &mut clusters);
    assert_eq!(
        rows(&grid, &clusters, 5),
        vec![
            "Shortcuts",
            "  Ctrl+Shift+/, F1        Show shortcuts",
            "  Ctrl+Shift+P            Open command palette",
            "  Q                       Quit",
            "",
        ]
    );
}

#[test]
fn palette_snapshot_matches_expected_rows_with_empty_query() {
    let keymap = registry();
    let mut grid = Grid::new(40, 5);
    let mut clusters = ClusterTable::new();
    render_palette(
        &keymap,
        "",
        0,
        &mut grid,
        Rect::new(0, 0, 40, 5),
        &mut clusters,
    );
    assert_eq!(
        rows(&grid, &clusters, 4),
        vec![">", "> Show shortcuts", "  Open command palette", "  Quit",]
    );
}
