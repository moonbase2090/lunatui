// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::{Cell, ClusterTable, Color, Grid, Rect, Size, Style};
use lunatui_runtime::{Clock, Handle, Outcome, TYPEAHEAD_TIMEOUT};
use lunatui_term::{Event, KeyCode, KeyEvent};
use lunatui_theme::{Theme, Token, Variant};
use lunatui_widgets::{FileEntry, FilePicker, FilePickerState, RenderCx, Widget, WidgetState};

const FILES: [FileEntry<'static>; 5] = [
    FileEntry::directory("docs").modified("Oct 30 09:12"),
    FileEntry::directory("src").modified("Nov 02 17:40"),
    FileEntry::file("Cargo.toml", 1_432).modified("Nov 01 08:03"),
    FileEntry::file("README.md", 18_204).modified("Oct 31 14:02"),
    FileEntry::file("release.tar.gz", 5_242_880).modified("Nov 02 22:11"),
];

fn grid(picker: &FilePicker<'_>, width: u16, height: u16) -> (Grid, ClusterTable) {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(width, height);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    picker.render(Rect::new(0, 0, width, height), &mut cx);
    (grid, clusters)
}

fn rows(picker: &FilePicker<'_>, width: u16, height: u16) -> Vec<String> {
    let (grid, clusters) = grid(picker, width, height);
    (0..height)
        .map(|y| grid.row_display(y, &clusters).trim_end().to_owned())
        .collect()
}

fn style_at(grid: &Grid, x: u16, y: u16) -> Style {
    grid.get(x, y).map(Cell::style).unwrap_or_default()
}

fn color(token: Token) -> Color {
    Theme::builtin(Variant::Dark).resolve(token)
}

#[test]
fn draws_the_path_filter_header_entries_and_count() {
    let picker = FilePicker::new("/home/dev/projects/lunatui", &FILES);
    assert_eq!(
        rows(&picker, 56, 9),
        [
            " ▸ /home/dev/projects/lunatui",
            " Filter type to narrow the list",
            "    Name                             Size  Modified",
            "  ▸ docs/                                  Oct 30 09:12",
            "  ▸ src/                                   Nov 02 17:40",
            "    Cargo.toml                     1.4 KB  Nov 01 08:03",
            "    README.md                       18 KB  Oct 31 14:02",
            "    release.tar.gz                 5.0 MB  Nov 02 22:11",
            " 5 items",
        ]
    );
}

#[test]
fn the_active_row_is_filled_with_the_accent_and_other_rows_are_not() {
    let picker = FilePicker::new("/a", &FILES).active(Some(2));
    let (cells, _) = grid(&picker, 56, 9);
    assert_eq!(style_at(&cells, 10, 5).bg, color(Token::Accent));
    assert_eq!(style_at(&cells, 10, 5).fg, color(Token::Surface0));
    assert_eq!(style_at(&cells, 55, 5).bg, color(Token::Accent));
    assert_eq!(style_at(&cells, 10, 6).bg, color(Token::Surface0));
    assert_eq!(style_at(&cells, 10, 4).bg, color(Token::Surface0));
}

#[test]
fn directories_are_accented_and_files_are_plain() {
    let picker = FilePicker::new("/a", &FILES);
    let (cells, _) = grid(&picker, 56, 9);
    assert_eq!(style_at(&cells, 4, 3).fg, color(Token::Accent));
    assert_eq!(style_at(&cells, 4, 5).fg, color(Token::TextPrimary));
}

#[test]
fn marked_rows_get_a_check_and_the_count_says_how_many_are_selected() {
    let marked = [2, 4];
    let picker = FilePicker::new("/a", &FILES).marked(&marked);
    let shown = rows(&picker, 56, 9);
    assert!(shown[5].starts_with(" ✓ "));
    assert!(shown[7].starts_with(" ✓ "));
    assert!(!shown[6].contains('✓'));
    assert_eq!(shown[8], " 5 items  ·  2 selected");
    let (cells, _) = grid(&picker, 56, 9);
    assert!(style_at(&cells, 5, 5).attrs.bold());
    assert!(!style_at(&cells, 5, 6).attrs.bold());
}

#[test]
fn a_single_entry_says_item_not_items() {
    let one = [FileEntry::file("a.txt", 3)];
    let picker = FilePicker::new("/a", &one);
    assert_eq!(rows(&picker, 40, 5)[4], " 1 item");
}

#[test]
fn sizes_are_rounded_into_units() {
    let entries = [
        FileEntry::file("a", 0),
        FileEntry::file("b", 1023),
        FileEntry::file("c", 1024),
        FileEntry::file("d", 1_536),
        FileEntry::file("e", 10_240),
        FileEntry::file("f", 18_204),
        FileEntry::file("g", 5_242_880),
        FileEntry::file("h", 3 * 1024 * 1024 * 1024),
    ];
    let picker = FilePicker::new("/a", &entries);
    let sizes: Vec<String> = rows(&picker, 40, 12)[3..11]
        .iter()
        .map(|row| row[row.len() - 8..].trim().to_owned())
        .collect();
    assert_eq!(
        sizes,
        [
            "0 B", "1023 B", "1.0 KB", "1.5 KB", "10 KB", "18 KB", "5.0 MB", "3.0 GB"
        ]
    );
}

#[test]
fn narrow_areas_drop_the_modified_column_then_the_size_column() {
    let picker = FilePicker::new("/a", &FILES);
    let mid = rows(&picker, 36, 9);
    assert_eq!(mid[2], "    Name                       Size");
    assert!(mid[5].ends_with("1.4 KB"));
    assert!(!mid[5].contains("Nov"));
    let small = rows(&picker, 24, 9);
    assert_eq!(small[2], "    Name");
    assert_eq!(small[5], "    Cargo.toml");
}

#[test]
fn a_long_path_keeps_its_end_behind_an_ellipsis() {
    let picker = FilePicker::new("/home/dev/projects/lunatui", &FILES);
    assert_eq!(rows(&picker, 24, 9)[0], " ▸ …dev/projects/lunatui");
    assert_eq!(rows(&picker, 56, 9)[0], " ▸ /home/dev/projects/lunatui");
}

#[test]
fn the_filter_text_replaces_the_hint_and_focus_adds_a_cursor() {
    let picker = FilePicker::new("/a", &FILES).filter("re");
    assert_eq!(rows(&picker, 40, 6)[1], " Filter re");
    let focused = picker.clone().state(WidgetState::Focused);
    assert_eq!(rows(&focused, 40, 6)[1], " Filter re▏");
    let empty = FilePicker::new("/a", &FILES).state(WidgetState::Focused);
    assert!(rows(&empty, 40, 6)[1].starts_with(" Filter ▏type to narrow"));
}

#[test]
fn the_list_scrolls_to_keep_the_active_row_in_view_and_shows_a_thumb() {
    let names: Vec<String> = (0..20).map(|n| format!("file{n:02}.txt")).collect();
    let entries: Vec<FileEntry<'_>> = names.iter().map(|n| FileEntry::file(n, 10)).collect();
    let top = FilePicker::new("/a", &entries).active(Some(0));
    let shown = rows(&top, 40, 8);
    assert!(shown[3].contains("file00.txt"));
    assert!(shown[3].ends_with('┃'));
    assert!(shown[6].ends_with('│'));
    let bottom = FilePicker::new("/a", &entries).active(Some(19));
    let shown = rows(&bottom, 40, 8);
    assert!(shown[3].contains("file16.txt"));
    assert!(shown[6].contains("file19.txt"));
    assert!(shown[3].ends_with('│'));
    assert!(shown[6].ends_with('┃'));
}

#[test]
fn no_thumb_is_drawn_when_every_entry_fits() {
    let picker = FilePicker::new("/a", &FILES).active(Some(0));
    assert!(!rows(&picker, 40, 9).iter().any(|row| row.contains('┃')));
}

#[test]
fn an_empty_folder_says_so() {
    let picker = FilePicker::new("/a", &[]);
    assert_eq!(rows(&picker, 40, 6)[3], "    This folder is empty");
    assert_eq!(rows(&picker, 40, 6)[5], " 0 items");
}

#[test]
fn a_disabled_picker_dims_its_rows() {
    let picker = FilePicker::new("/a", &FILES).state(WidgetState::Disabled);
    let (cells, _) = grid(&picker, 56, 9);
    assert!(style_at(&cells, 10, 5).attrs.dim());
    assert!(style_at(&cells, 3, 0).attrs.dim());
}

#[test]
fn the_active_row_past_the_last_entry_is_ignored() {
    let picker = FilePicker::new("/a", &FILES).active(Some(99));
    let (cells, _) = grid(&picker, 56, 9);
    assert_eq!(style_at(&cells, 10, 3).bg, color(Token::Surface0));
}

#[test]
fn too_small_areas_draw_nothing_and_do_not_panic() {
    let picker = FilePicker::new("/a", &FILES).active(Some(1));
    assert_eq!(rows(&picker, 40, 3), ["", "", ""]);
    assert_eq!(rows(&picker, 0, 0), Vec::<String>::new());
    let _ = rows(&picker, 1, 4);
    let _ = rows(&picker, 3, 5);
}

#[test]
fn hit_testing_maps_rows_to_entries_after_scrolling() {
    let names: Vec<String> = (0..20).map(|n| format!("f{n}")).collect();
    let entries: Vec<FileEntry<'_>> = names.iter().map(|n| FileEntry::file(n, 1)).collect();
    let picker = FilePicker::new("/a", &entries).active(Some(19));
    let area = Rect::new(2, 1, 40, 8);
    assert_eq!(picker.list_area(area), Rect::new(2, 4, 40, 4));
    assert_eq!(picker.entry_at(area, 5, 4), Some(16));
    assert_eq!(picker.entry_at(area, 5, 7), Some(19));
    assert_eq!(picker.entry_at(area, 5, 8), None);
    assert_eq!(picker.entry_at(area, 5, 3), None);
    assert_eq!(picker.entry_at(area, 1, 4), None);
    assert_eq!(picker.entry_at(area, 42, 4), None);
}

#[test]
fn step_moves_by_delta_and_stops_at_the_ends() {
    assert_eq!(FilePicker::step(None, 5, 1), Some(0));
    assert_eq!(FilePicker::step(None, 5, -1), Some(0));
    assert_eq!(FilePicker::step(Some(1), 5, 1), Some(2));
    assert_eq!(FilePicker::step(Some(1), 5, -1), Some(0));
    assert_eq!(FilePicker::step(Some(0), 5, -1), Some(0));
    assert_eq!(FilePicker::step(Some(4), 5, 1), Some(4));
    assert_eq!(FilePicker::step(Some(1), 5, 10), Some(4));
    assert_eq!(FilePicker::step(Some(3), 5, -10), Some(0));
    assert_eq!(FilePicker::step(Some(9), 5, -1), Some(0));
    assert_eq!(FilePicker::step(Some(0), 0, 1), None);
}

fn key(code: KeyCode) -> Event {
    Event::Key(KeyEvent::press(code))
}

#[test]
fn typing_jumps_to_names_and_shares_the_type_ahead_timeout() {
    let mut clock = Clock::new();
    let mut state = FilePickerState::new();
    state.sync(&FILES, 3);
    let typed = |state: &mut FilePickerState, clock: &Clock, ch| {
        state.set_time(clock.now());
        state.handle(&key(KeyCode::Char(ch)))
    };
    assert_eq!(typed(&mut state, &clock, 'c'), Outcome::Changed);
    assert_eq!(state.focused(), Some(2));
    clock.advance(TYPEAHEAD_TIMEOUT);
    assert_eq!(typed(&mut state, &clock, 'R'), Outcome::Changed);
    assert_eq!(state.focused(), Some(3));
    assert_eq!(typed(&mut state, &clock, 'r'), Outcome::Changed);
    assert_eq!(state.focused(), Some(4), "the same letter moves on");
    assert_eq!(typed(&mut state, &clock, 'r'), Outcome::Changed);
    assert_eq!(state.focused(), Some(3), "and wraps");
    clock.advance(TYPEAHEAD_TIMEOUT);
    for ch in "rel".chars() {
        typed(&mut state, &clock, ch);
    }
    assert_eq!(state.focused(), Some(4), "a burst builds one query");
    assert_eq!(typed(&mut state, &clock, 'z'), Outcome::Handled);
}

#[test]
fn space_marks_rows_enter_opens_and_paging_stops_at_the_ends() {
    let mut state = FilePickerState::new();
    state.sync(&FILES, 2);
    assert_eq!(state.handle(&key(KeyCode::Down)), Outcome::Changed);
    assert_eq!(state.handle(&key(KeyCode::Char(' '))), Outcome::Toggled(0));
    assert_eq!(state.handle(&key(KeyCode::PageDown)), Outcome::Changed);
    assert_eq!(state.focused(), Some(2));
    assert_eq!(state.handle(&key(KeyCode::Char(' '))), Outcome::Toggled(2));
    assert_eq!(state.marked(), &[0, 2]);
    assert_eq!(state.handle(&key(KeyCode::PageDown)), Outcome::Changed);
    assert_eq!(state.handle(&key(KeyCode::PageDown)), Outcome::Handled);
    assert_eq!(state.focused(), Some(4));
    assert_eq!(state.handle(&key(KeyCode::Enter)), Outcome::Activated(4));
    assert_eq!(state.handle(&key(KeyCode::Home)), Outcome::Changed);
    assert_eq!(state.focused(), Some(0));
    state.sync(&FILES[..1], 2);
    assert_eq!(state.marked(), &[0], "marks past the end are dropped");
}

#[test]
fn toggle_keeps_the_marks_sorted_and_flips_membership() {
    let mut marked = Vec::new();
    FilePicker::toggle(&mut marked, 3);
    FilePicker::toggle(&mut marked, 1);
    FilePicker::toggle(&mut marked, 2);
    assert_eq!(marked, [1, 2, 3]);
    FilePicker::toggle(&mut marked, 2);
    assert_eq!(marked, [1, 3]);
}

#[test]
fn parent_drops_the_last_path_segment() {
    assert_eq!(FilePicker::parent("/home/dev/projects"), "/home/dev");
    assert_eq!(FilePicker::parent("/home/dev/projects/"), "/home/dev");
    assert_eq!(FilePicker::parent("/home"), "/");
    assert_eq!(FilePicker::parent("/"), "/");
    assert_eq!(FilePicker::parent("relative"), "relative");
    assert_eq!(FilePicker::parent("a/b"), "a");
}

#[test]
fn matches_ignores_case_and_accepts_an_empty_query() {
    assert!(FilePicker::matches("README.md", "read"));
    assert!(FilePicker::matches("README.md", ".MD"));
    assert!(FilePicker::matches("README.md", ""));
    assert!(!FilePicker::matches("README.md", "toml"));
}

#[test]
fn measure_wants_the_header_rows_the_entries_and_the_count_line() {
    let picker = FilePicker::new("/a", &FILES);
    assert_eq!(picker.measure(Size::new(60, 30)), Size::new(60, 9));
    assert_eq!(picker.measure(Size::new(60, 5)), Size::new(60, 5));
    assert_eq!(picker.measure(Size::new(0, 5)), Size::new(0, 0));
    assert_eq!(
        FilePicker::new("/a", &[]).measure(Size::new(60, 30)),
        Size::new(60, 5)
    );
}
