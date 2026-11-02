// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::{Cell, ClusterTable, Grid, Rect, Style};
use lunatui_theme::{Theme, Token, Variant};
use lunatui_widgets::{CommandPalette, PaletteEntry, RenderCx, SearchBox, Widget};

const ENTRIES: [PaletteEntry<'static>; 5] = [
    PaletteEntry::new("Open file").shortcut("Ctrl+O"),
    PaletteEntry::new("Save file")
        .shortcut("Ctrl+S")
        .detail("write to disk"),
    PaletteEntry::new("Close tab").shortcut("Ctrl+W"),
    PaletteEntry::new("Split pane right"),
    PaletteEntry::new("Toggle sidebar").shortcut("Ctrl+B"),
];

fn draw(palette: &CommandPalette<'_>, width: u16, height: u16) -> (Grid, ClusterTable) {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(width, height);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    palette.render(Rect::new(0, 0, width, height), &mut cx);
    (grid, clusters)
}

fn text(palette: &CommandPalette<'_>) -> String {
    let (grid, clusters) = draw(palette, 80, 20);
    grid.display_text(&clusters)
}

fn style_at(grid: &Grid, x: u16, y: u16) -> Style {
    grid.get(x, y).map(Cell::style).unwrap_or_default()
}

fn token(token: Token) -> lunatui_core::Color {
    Theme::builtin(Variant::Dark).resolve(token)
}

fn find(grid: &Grid, clusters: &ClusterTable, needle: &str) -> Option<(u16, u16)> {
    (0..grid.height()).find_map(|y| {
        let row = grid.row_display(y, clusters);
        let at = row.find(needle)?;
        Some((u16::try_from(row[..at].chars().count()).ok()?, y))
    })
}

#[test]
fn an_empty_query_lists_every_entry_with_shortcuts() {
    let out = text(&CommandPalette::new("", 0, &ENTRIES));
    for entry in &ENTRIES {
        assert!(out.contains(entry.title()), "{}", entry.title());
    }
    assert!(out.contains("Ctrl+S") && out.contains("write to disk"));
    assert!(out.contains("Type a command") && out.contains("5 commands"));
}

#[test]
fn the_query_filters_and_counts_matches() {
    let palette = CommandPalette::new("file", 4, &ENTRIES);
    let out = text(&palette);
    assert!(out.contains("Open file") && out.contains("Save file"));
    assert!(!out.contains("Close tab"));
    assert!(out.contains("2 commands"));
    assert_eq!(palette.matches().len(), 2);
    assert_eq!(palette.matches()[1].index, 1);
}

#[test]
fn no_match_says_so() {
    let out = text(&CommandPalette::new("zzz", 3, &ENTRIES));
    assert!(out.contains("No matching commands"));
    assert!(out.contains("0 commands"));
}

#[test]
fn the_selected_row_is_filled_with_the_accent() {
    let palette = CommandPalette::new("", 0, &ENTRIES).selected(Some(2));
    let (grid, clusters) = draw(&palette, 80, 20);
    let (x, y) = find(&grid, &clusters, "Close tab").expect("row drawn");
    assert_eq!(style_at(&grid, x, y).bg, token(Token::Accent));
    let (x, y) = find(&grid, &clusters, "Open file").expect("row drawn");
    assert_ne!(style_at(&grid, x, y).bg, token(Token::Accent));
}

#[test]
fn matched_text_is_highlighted() {
    let palette = CommandPalette::new("sav", 3, &ENTRIES);
    let (grid, clusters) = draw(&palette, 80, 20);
    let (x, y) = find(&grid, &clusters, "Save").expect("row drawn");
    assert_eq!(style_at(&grid, x, y).fg, token(Token::Accent));
    assert_ne!(style_at(&grid, x + 3, y).fg, token(Token::Accent));
}

#[test]
fn rows_scroll_to_keep_the_selection_visible() {
    let palette = CommandPalette::new("", 0, &ENTRIES)
        .max_rows(3)
        .selected(Some(4));
    let out = text(&palette);
    assert!(out.contains("Toggle sidebar") && out.contains("Close tab"));
    assert!(!out.contains("Open file") && !out.contains("Save file"));
    let top = CommandPalette::new("", 0, &ENTRIES)
        .max_rows(3)
        .selected(Some(0));
    let out = text(&top);
    assert!(out.contains("Open file") && !out.contains("Toggle sidebar"));
}

#[test]
fn clicks_map_to_positions_among_the_matches() {
    let area = Rect::new(0, 0, 80, 20);
    let palette = CommandPalette::new("file", 4, &ENTRIES);
    let (grid, clusters) = draw(&palette, 80, 20);
    let (x, y) = find(&grid, &clusters, "Save file").expect("row drawn");
    assert_eq!(palette.entry_at(area, x, y), Some(1));
    assert_eq!(palette.entry_at(area, x, y + 1), None);
    assert_eq!(palette.entry_at(area, 0, 0), None);
    let scrolled = CommandPalette::new("", 0, &ENTRIES)
        .max_rows(3)
        .selected(Some(4));
    let (grid, clusters) = draw(&scrolled, 80, 20);
    let (x, y) = find(&grid, &clusters, "Toggle sidebar").expect("row drawn");
    assert_eq!(scrolled.entry_at(area, x, y), Some(4));
}

#[test]
fn stepping_wraps_through_the_matches() {
    let total = CommandPalette::new("file", 4, &ENTRIES).matches().len();
    assert_eq!(SearchBox::step(None, total, true), Some(0));
    assert_eq!(SearchBox::step(Some(1), total, true), Some(0));
    assert_eq!(SearchBox::step(Some(0), total, false), Some(1));
}

#[test]
fn the_panel_is_centred_and_fits_a_small_area() {
    let area = Rect::new(0, 0, 80, 20);
    let panel = CommandPalette::new("", 0, &ENTRIES).area(area);
    assert_eq!(panel.width, 62);
    assert_eq!(panel.x * 2 + panel.width, 80);
    let small = Rect::new(0, 0, 30, 6);
    let panel = CommandPalette::new("", 0, &ENTRIES).area(small);
    assert!(panel.x + panel.width <= 30 && panel.y + panel.height <= 6);
    let (grid, clusters) = draw(&CommandPalette::new("sa", 2, &ENTRIES), 30, 6);
    assert!(grid.display_text(&clusters).contains('╭'));
}

#[test]
fn a_zero_sized_area_draws_nothing() {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(4, 4);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    CommandPalette::new("", 0, &ENTRIES).render(Rect::new(0, 0, 0, 0), &mut cx);
    assert_eq!(grid.display_text(&clusters), "    \n    \n    \n    ");
}

#[test]
fn fuzzy_matching_finds_ordered_characters_and_their_byte_ranges() {
    let palette = CommandPalette::new("spt", 3, &ENTRIES);
    let found = palette.matches();
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].index, 3);
    assert_eq!(found[0].ranges, [0..1, 1..2, 4..5]);
    assert!(CommandPalette::new("tps", 3, &ENTRIES).matches().is_empty());
}

#[test]
fn matching_ignores_ascii_case_unless_query_contains_uppercase() {
    let entries = [PaletteEntry::new("Save"), PaletteEntry::new("save")];
    assert_eq!(CommandPalette::new("sv", 2, &entries).matches().len(), 2);
    let hits = CommandPalette::new("Sv", 2, &entries).matches();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].index, 0);
}

#[test]
fn details_and_shortcuts_are_not_command_search_text() {
    assert!(
        CommandPalette::new("disk", 4, &ENTRIES)
            .matches()
            .is_empty()
    );
    assert!(
        CommandPalette::new("Ctrl", 4, &ENTRIES)
            .matches()
            .is_empty()
    );
    assert_eq!(
        CommandPalette::new("s p t", 5, &ENTRIES).matches()[0].index,
        3
    );
}

#[test]
fn unicode_matches_highlight_whole_graphemes() {
    let entries = [PaletteEntry::new("Cafe\u{301} 界")];
    let hits = CommandPalette::new("e界", 4, &entries).matches();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].ranges, [3..6, 7..10]);
    let (grid, clusters) = draw(&CommandPalette::new("e", 1, &entries), 80, 20);
    let (x, y) = find(&grid, &clusters, "Cafe").unwrap();
    assert_eq!(style_at(&grid, x + 3, y).fg, token(Token::Accent));
}

#[test]
fn the_query_scrolls_to_keep_its_caret_visible() {
    let palette = CommandPalette::new("abcdefghijklmnopqrstuvwxyz", 26, &ENTRIES).width(22);
    let out = text(&palette);
    assert!(out.contains("xyz"), "{out}");
    assert!(!out.contains("abcd"));
    let (grid, _) = draw(&palette, 80, 20);
    let caret_cells = (0..grid.height())
        .flat_map(|y| (0..grid.width()).map(move |x| (x, y)))
        .filter(|&(x, y)| style_at(&grid, x, y).bg == token(Token::FocusRing))
        .count();
    assert_eq!(caret_cells, 1);
}

#[test]
fn caret_inside_an_emoji_or_combining_sequence_keeps_the_whole_grapheme() {
    for (query, caret, needle) in [("👩‍💻 run", 5, "👩‍💻"), ("e\u{301}cho", 1, "e\u{301}")]
    {
        let palette = CommandPalette::new(query, caret, &ENTRIES);
        let (grid, clusters) = draw(&palette, 80, 20);
        let (x, y) = find(&grid, &clusters, needle).expect("whole grapheme drawn");
        assert_eq!(style_at(&grid, x, y).bg, token(Token::FocusRing));
    }
}

#[test]
fn click_geometry_respects_nonzero_origins_and_panel_borders() {
    let palette = CommandPalette::new("", 0, &ENTRIES)
        .max_rows(2)
        .selected(Some(4));
    let area = Rect::new(8, 6, 80, 20);
    let panel = palette.area(area);
    assert_eq!(palette.entry_at(area, panel.x + 1, panel.y + 2), Some(3));
    assert_eq!(palette.entry_at(area, panel.x + 1, panel.y + 3), Some(4));
    assert_eq!(palette.entry_at(area, panel.x, panel.y + 3), None);
    assert_eq!(palette.entry_at(area, panel.x + 1, panel.y + 1), None);
    assert_eq!(palette.entry_at(area, panel.x + 1, panel.y + 4), None);
}

#[test]
fn out_of_range_selection_returns_to_the_top_without_highlighting() {
    let palette = CommandPalette::new("", 0, &ENTRIES)
        .max_rows(2)
        .selected(Some(usize::MAX));
    let out = text(&palette);
    assert!(out.contains("Open file") && out.contains("Save file"));
    assert!(!out.contains("Toggle sidebar"));
    let (grid, clusters) = draw(&palette, 80, 20);
    let (x, y) = find(&grid, &clusters, "Open file").unwrap();
    assert_ne!(style_at(&grid, x, y).bg, token(Token::Accent));
}

#[test]
fn oversized_hints_and_tiny_panels_do_not_escape_the_panel() {
    let shortcut = "x".repeat(70000);
    let entries = [PaletteEntry::new("界界界").shortcut(&shortcut)];
    for width in 1..8 {
        let theme = Theme::builtin(Variant::Dark);
        let mut grid = Grid::new(12, 6);
        let mut clusters = ClusterTable::new();
        let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
        CommandPalette::new("界", 3, &entries)
            .width(width)
            .render(Rect::new(2, 2, width, 2), &mut cx);
        assert_eq!(grid.row_display(0, &clusters), "            ");
        assert_eq!(grid.row_display(5, &clusters), "            ");
    }
}

#[test]
fn title_border_placeholder_and_width_are_configurable() {
    let palette = CommandPalette::new("", 0, &ENTRIES)
        .title("Run command")
        .placeholder("Find an action")
        .border_type(lunatui_widgets::BorderType::Plain)
        .backdrop(0.0)
        .width(50)
        .max_rows(0);
    let out = text(&palette);
    assert!(out.contains("Run command") && out.contains("Find an action"));
    assert!(out.contains('┌') && out.contains("Open file"));
    assert!(!out.contains("Save file"));
    assert_eq!(palette.area(Rect::new(0, 0, 80, 20)).width, 50);
}
