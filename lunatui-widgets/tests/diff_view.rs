// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::{Cell, ClusterTable, Color, Grid, Rect, Size, Style};
use lunatui_theme::{Theme, Token, Variant};
use lunatui_widgets::{Diff, DiffKind, DiffLayout, DiffView, RenderCx, Widget, WidgetState};

const SAMPLE: &str = "--- a/src/main.rs\n+++ b/src/main.rs\n@@ -1,5 +1,6 @@ fn main\n use std::io;\n-let x = 1;\n-let y = 2;\n+let x = 10;\n+let y = 2;\n+let z = 3;\n println!(\"hi\");\n end\n";

fn draw(view: &DiffView<'_>, width: u16, height: u16) -> (Grid, ClusterTable) {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(width, height);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    view.render(Rect::new(0, 0, width, height), &mut cx);
    (grid, clusters)
}

fn rows(view: &DiffView<'_>, width: u16, height: u16) -> Vec<String> {
    let (grid, clusters) = draw(view, width, height);
    (0..height)
        .map(|y| grid.row_display(y, &clusters))
        .collect()
}

fn padded<const N: usize>(width: usize, rows: [&str; N]) -> Vec<String> {
    rows.map(|row| {
        let pad = width.saturating_sub(row.chars().count());
        format!("{row}{}", " ".repeat(pad))
    })
    .to_vec()
}

fn style_at(grid: &Grid, x: u16, y: u16) -> Style {
    grid.get(x, y).map(Cell::style).unwrap_or_default()
}

fn color(token: Token) -> Color {
    Theme::builtin(Variant::Dark).resolve(token)
}

#[test]
fn parsing_reads_names_hunks_and_lines() {
    let diff = Diff::parse(SAMPLE);
    assert_eq!(diff.old_name(), "a/src/main.rs");
    assert_eq!(diff.new_name(), "b/src/main.rs");
    assert_eq!(diff.hunks().len(), 1);
    assert_eq!(diff.hunks()[0].header, "@@ -1,5 +1,6 @@ fn main");
    assert_eq!(diff.hunks()[0].lines.len(), 8);
}

#[test]
fn lines_get_their_kind_and_numbers() {
    let diff = Diff::parse(SAMPLE);
    let shown: Vec<(DiffKind, Option<u32>, Option<u32>, &str)> = diff.hunks()[0]
        .lines
        .iter()
        .map(|line| (line.kind, line.old, line.new, line.text.as_str()))
        .collect();
    assert_eq!(
        shown,
        [
            (DiffKind::Context, Some(1), Some(1), "use std::io;"),
            (DiffKind::Removed, Some(2), None, "let x = 1;"),
            (DiffKind::Removed, Some(3), None, "let y = 2;"),
            (DiffKind::Added, None, Some(2), "let x = 10;"),
            (DiffKind::Added, None, Some(3), "let y = 2;"),
            (DiffKind::Added, None, Some(4), "let z = 3;"),
            (DiffKind::Context, Some(4), Some(5), "println!(\"hi\");"),
            (DiffKind::Context, Some(5), Some(6), "end"),
        ]
    );
}

#[test]
fn stats_count_added_and_removed_lines() {
    assert_eq!(Diff::parse(SAMPLE).stats(), (3, 2));
    assert_eq!(Diff::parse("").stats(), (0, 0));
}

#[test]
fn every_hunk_restarts_numbering_from_its_header() {
    let diff = Diff::parse("@@ -10,2 +12,3 @@\n a\n+b\n c\n@@ -40 +50 @@\n-d\n+e\n");
    let first = &diff.hunks()[0].lines;
    assert_eq!((first[0].old, first[0].new), (Some(10), Some(12)));
    assert_eq!((first[1].old, first[1].new), (None, Some(13)));
    assert_eq!((first[2].old, first[2].new), (Some(11), Some(14)));
    let second = &diff.hunks()[1].lines;
    assert_eq!((second[0].old, second[0].new), (Some(40), None));
    assert_eq!((second[1].old, second[1].new), (None, Some(50)));
}

#[test]
fn a_malformed_header_starts_numbering_at_one() {
    let diff = Diff::parse("@@ nonsense @@\n a\n");
    assert_eq!(diff.hunks()[0].lines[0].old, Some(1));
    assert_eq!(diff.hunks()[0].lines[0].new, Some(1));
}

#[test]
fn preamble_lines_and_no_newline_markers_are_skipped() {
    let diff = Diff::parse(
        "diff --git a/f b/f\nindex 1..2 100644\n--- a/f\t2026\n+++ b/f\n@@ -1 +1 @@\n-old\n\\ No newline at end of file\n+new\n",
    );
    assert_eq!(diff.old_name(), "a/f");
    assert_eq!(diff.hunks()[0].lines.len(), 2);
}

#[test]
fn an_empty_line_in_a_hunk_is_empty_context() {
    let diff = Diff::parse("@@ -1,2 +1,2 @@\n a\n\n");
    let line = &diff.hunks()[0].lines[1];
    assert_eq!(line.kind, DiffKind::Context);
    assert_eq!(line.text, "");
    assert_eq!(line.old, Some(2));
}

#[test]
fn dashes_inside_a_hunk_are_removed_text() {
    let diff = Diff::parse("@@ -1 +1 @@\n--- not a header\n+++ nor this\n");
    let lines = &diff.hunks()[0].lines;
    assert_eq!(lines[0].kind, DiffKind::Removed);
    assert_eq!(lines[0].text, "-- not a header");
    assert_eq!(lines[1].kind, DiffKind::Added);
}

#[test]
fn the_unified_layout_lists_removed_then_added_lines() {
    let diff = Diff::parse(SAMPLE);
    assert_eq!(
        rows(&DiffView::new(&diff), 24, 9),
        padded(
            24,
            [
                "@@ -1,5 +1,6 @@ fn main",
                "1 1   use std::io;",
                "2   - let x = 1;",
                "3   - let y = 2;",
                "  2 + let x = 10;",
                "  3 + let y = 2;",
                "  4 + let z = 3;",
                "4 5   println!(\"hi\");",
                "5 6   end",
            ]
        )
    );
}

#[test]
fn the_side_by_side_layout_pairs_changes() {
    let diff = Diff::parse(SAMPLE);
    let view = DiffView::new(&diff).layout(DiffLayout::SideBySide);
    assert_eq!(
        rows(&view, 41, 7),
        padded(
            41,
            [
                "@@ -1,5 +1,6 @@ fn main",
                "1   use std::io;    │1   use std::io;",
                "2 - let x = 1;      │2 + let x = 10;",
                "3 - let y = 2;      │3 + let y = 2;",
                "                    │4 + let z = 3;",
                "4   println!(\"hi\"); │5   println!(\"hi\");",
                "5   end             │6   end",
            ]
        )
    );
}

#[test]
fn removed_lines_without_a_partner_leave_the_right_side_blank() {
    let diff = Diff::parse("@@ -1,3 +1,1 @@\n-a\n-b\n+c\n");
    let view = DiffView::new(&diff).layout(DiffLayout::SideBySide);
    assert_eq!(
        rows(&view, 21, 3),
        padded(21, ["@@ -1,3 +1,1 @@", "1 - a     │1 + c", "2 - b     │"])
    );
}

#[test]
fn row_counts_depend_on_the_layout() {
    let diff = Diff::parse(SAMPLE);
    assert_eq!(DiffView::new(&diff).rows(), 9);
    assert_eq!(
        DiffView::new(&diff).layout(DiffLayout::SideBySide).rows(),
        7
    );
    assert_eq!(DiffView::new(&diff).max_scroll(4), 5);
    assert_eq!(DiffView::new(&diff).max_scroll(20), 0);
}

#[test]
fn scrolling_starts_at_a_later_row_and_stops_at_the_end() {
    let diff = Diff::parse(SAMPLE);
    let view = DiffView::new(&diff).scroll(7);
    assert_eq!(
        rows(&view, 24, 2),
        padded(24, ["4 5   println!(\"hi\");", "5 6   end"])
    );
    let past = DiffView::new(&diff).scroll(99);
    assert_eq!(rows(&past, 24, 2), rows(&view, 24, 2));
    let top = DiffView::new(&diff).scroll(1);
    assert_eq!(rows(&top, 24, 1)[0], padded(24, ["1 1   use std::io;"])[0]);
}

#[test]
fn line_numbers_can_be_turned_off() {
    let diff = Diff::parse(SAMPLE);
    let view = DiffView::new(&diff).line_numbers(false);
    assert_eq!(
        rows(&view, 16, 4),
        padded(
            16,
            [
                "@@ -1,5 +1,6 @@",
                "  use std::io;",
                "- let x = 1;",
                "- let y = 2;"
            ]
        )
    );
}

#[test]
fn number_columns_widen_for_big_line_numbers() {
    let diff = Diff::parse("@@ -98,2 +98,2 @@\n a\n-b\n+c\n");
    assert_eq!(
        rows(&DiffView::new(&diff), 20, 4),
        padded(
            20,
            ["@@ -98,2 +98,2 @@", "98 98   a", "99    - b", "   99 + c"]
        )
    );
}

#[test]
fn colors_follow_the_kind_of_line() {
    let diff = Diff::parse(SAMPLE);
    let (grid, _) = draw(&DiffView::new(&diff), 24, 9);
    assert_eq!(style_at(&grid, 6, 1).fg, color(Token::TextPrimary));
    assert_eq!(style_at(&grid, 6, 2).fg, color(Token::Fail));
    assert_eq!(style_at(&grid, 4, 2).fg, color(Token::Fail));
    assert_eq!(style_at(&grid, 6, 4).fg, color(Token::Ok));
    assert_eq!(style_at(&grid, 4, 4).fg, color(Token::Ok));
    assert!(style_at(&grid, 0, 2).attrs.dim());
    let header = style_at(&grid, 0, 0);
    assert_eq!(header.fg, color(Token::Accent));
    assert_eq!(header.bg, color(Token::Surface1));
    assert_eq!(style_at(&grid, 23, 0).bg, color(Token::Surface1));
    assert_eq!(style_at(&grid, 23, 1).bg, color(Token::Surface0));
}

#[test]
fn the_changed_part_of_a_paired_line_is_inverted() {
    let diff = Diff::parse(SAMPLE);
    let (grid, _) = draw(&DiffView::new(&diff), 24, 9);
    let hit = style_at(&grid, 15, 4);
    assert_eq!(hit.bg, color(Token::Ok));
    assert_eq!(hit.fg, color(Token::Surface0));
    assert!(hit.attrs.bold());
    assert_eq!(style_at(&grid, 14, 4).bg, color(Token::Surface0));
    assert_eq!(style_at(&grid, 16, 4).bg, color(Token::Surface0));
    assert_eq!(style_at(&grid, 12, 2).bg, color(Token::Surface0));
}

#[test]
fn identical_paired_text_highlights_nothing() {
    let diff = Diff::parse(SAMPLE);
    let (grid, _) = draw(&DiffView::new(&diff), 24, 9);
    for x in 6..16 {
        assert_eq!(style_at(&grid, x, 3).bg, color(Token::Surface0), "{x}");
        assert_eq!(style_at(&grid, x, 5).bg, color(Token::Surface0), "{x}");
    }
}

#[test]
fn a_removed_line_marks_what_the_new_line_dropped() {
    let diff = Diff::parse("@@ -1 +1 @@\n-let alpha = 1;\n+let beta = 1;\n");
    let (grid, _) = draw(&DiffView::new(&diff), 24, 3);
    let lit: Vec<bool> = (6..18)
        .map(|x| style_at(&grid, x, 1).bg == color(Token::Fail))
        .collect();
    assert_eq!(
        lit,
        [
            false, false, false, false, true, true, true, true, false, false, false, false
        ]
    );
    let lit: Vec<bool> = (6..17)
        .map(|x| style_at(&grid, x, 2).bg == color(Token::Ok))
        .collect();
    assert_eq!(
        lit,
        [
            false, false, false, false, true, true, true, false, false, false, false
        ]
    );
}

#[test]
fn turning_off_intra_line_draws_whole_lines_plainly() {
    let diff = Diff::parse(SAMPLE);
    let view = DiffView::new(&diff).intra_line(false);
    let (grid, _) = draw(&view, 24, 9);
    for x in 6..17 {
        assert_eq!(style_at(&grid, x, 4).bg, color(Token::Surface0), "{x}");
    }
}

#[test]
fn multibyte_text_is_split_on_character_boundaries() {
    let diff = Diff::parse("@@ -1 +1 @@\n-héllo wörld\n+hello wörld\n");
    let (grid, clusters) = draw(&DiffView::new(&diff), 24, 3);
    assert_eq!(grid.row_display(1, &clusters), "1   - héllo wörld       ");
    assert_eq!(style_at(&grid, 7, 1).bg, color(Token::Fail));
    assert_eq!(style_at(&grid, 8, 1).bg, color(Token::Surface0));
    assert_eq!(style_at(&grid, 7, 2).bg, color(Token::Ok));
}

#[test]
fn side_by_side_pairs_get_their_own_highlights() {
    let diff = Diff::parse(SAMPLE);
    let view = DiffView::new(&diff).layout(DiffLayout::SideBySide);
    let (grid, _) = draw(&view, 41, 7);
    assert_eq!(style_at(&grid, 4 + 9, 2).bg, color(Token::Surface0));
    assert_eq!(style_at(&grid, 21 + 4 + 9, 2).bg, color(Token::Ok));
    assert_eq!(style_at(&grid, 20, 2).fg, color(Token::TextPrimary));
}

#[test]
fn disabled_dims_every_row() {
    let diff = Diff::parse(SAMPLE);
    let view = DiffView::new(&diff).state(WidgetState::Disabled);
    let (grid, _) = draw(&view, 24, 9);
    for y in 0..9 {
        assert!(style_at(&grid, 1, y).attrs.dim(), "gutter {y}");
        assert!(style_at(&grid, 8, y).attrs.dim(), "text {y}");
    }
}

#[test]
fn long_lines_are_clipped_to_the_width() {
    let diff = Diff::parse(SAMPLE);
    assert_eq!(
        rows(&DiffView::new(&diff), 12, 3),
        padded(12, ["@@ -1,5 +1,6", "1 1   use st", "2   - let x "])
    );
    assert_eq!(rows(&DiffView::new(&diff), 3, 2), padded(3, ["@@ ", "1 1"]));
}

#[test]
fn rows_past_the_diff_are_blank_background() {
    let diff = Diff::parse("@@ -1 +1 @@\n a\n");
    let (grid, clusters) = draw(&DiffView::new(&diff), 12, 4);
    assert_eq!(grid.row_display(3, &clusters), " ".repeat(12));
    assert_eq!(style_at(&grid, 5, 3).bg, color(Token::Surface0));
}

#[test]
fn an_empty_diff_draws_blank_rows() {
    let diff = Diff::default();
    assert_eq!(DiffView::new(&diff).rows(), 0);
    assert_eq!(rows(&DiffView::new(&diff), 6, 2), padded(6, ["", ""]));
}

#[test]
fn measure_is_one_row_per_displayed_row_up_to_the_space() {
    let diff = Diff::parse(SAMPLE);
    let view = DiffView::new(&diff);
    assert_eq!(view.measure(Size::new(30, 20)), Size::new(30, 9));
    assert_eq!(view.measure(Size::new(30, 4)), Size::new(30, 4));
}

#[test]
fn a_zero_sized_area_draws_nothing() {
    let diff = Diff::parse(SAMPLE);
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(6, 2);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    DiffView::new(&diff).render(Rect::new(0, 0, 0, 2), &mut cx);
    DiffView::new(&diff).render(Rect::new(0, 0, 6, 0), &mut cx);
    assert_eq!(grid.row_display(0, &clusters), "      ");
}

#[test]
fn a_line_that_is_the_tail_of_its_partner_highlights_the_extra_start() {
    let diff = Diff::parse("@@ -1 +1 @@\n-abc\n+xabc\n");
    let (grid, _) = draw(&DiffView::new(&diff), 12, 3);
    for x in 6..9 {
        assert_eq!(style_at(&grid, x, 1).bg, color(Token::Surface0), "{x}");
    }
    assert_eq!(style_at(&grid, 6, 2).bg, color(Token::Ok));
    assert_eq!(style_at(&grid, 7, 2).bg, color(Token::Surface0));
}

#[test]
fn characters_sharing_only_their_last_byte_are_highlighted_whole() {
    let diff = Diff::parse("@@ -1 +1 @@\n-aé!\n+aũ!\n");
    let (grid, clusters) = draw(&DiffView::new(&diff), 12, 3);
    assert_eq!(grid.row_display(1, &clusters), "1   - aé!   ");
    assert_eq!(style_at(&grid, 7, 1).bg, color(Token::Fail));
    assert_eq!(style_at(&grid, 8, 1).bg, color(Token::Surface0));
    let diff = Diff::parse("@@ -1 +1 @@\n-aé\n+aũ\n");
    let (grid, clusters) = draw(&DiffView::new(&diff), 12, 3);
    assert_eq!(grid.row_display(2, &clusters), "  1 + aũ    ");
    assert_eq!(style_at(&grid, 7, 2).bg, color(Token::Ok));
}

#[test]
fn a_shorter_partner_that_is_a_prefix_highlights_the_rest() {
    let diff = Diff::parse("@@ -1 +1 @@\n-abc\n+abcde\n");
    let (grid, _) = draw(&DiffView::new(&diff), 12, 3);
    assert_eq!(style_at(&grid, 9, 2).bg, color(Token::Ok));
    assert_eq!(style_at(&grid, 10, 2).bg, color(Token::Ok));
    assert_eq!(style_at(&grid, 8, 2).bg, color(Token::Surface0));
}

#[test]
fn side_by_side_without_line_numbers_has_no_gutter() {
    let diff = Diff::parse("@@ -1 +1 @@\n a\n-b\n+c\n");
    let view = DiffView::new(&diff)
        .layout(DiffLayout::SideBySide)
        .line_numbers(false);
    assert_eq!(
        rows(&view, 15, 3),
        padded(15, ["@@ -1 +1 @@", "  a    │  a", "- b    │+ c"])
    );
}

fn overflow_rows(layout: DiffLayout) -> Vec<String> {
    let diff = Diff::parse(
        "@@ -1,2 +1,2 @@\n a very long context line that cannot fit\n-old long text that cannot fit either\n+new long text that cannot fit either\n",
    );
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(41, 4);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    DiffView::new(&diff)
        .layout(layout)
        .render(Rect::new(0, 0, 21, 4), &mut cx);
    (0..4).map(|y| grid.row_display(y, &clusters)).collect()
}

#[test]
fn unified_text_never_draws_past_the_area() {
    for row in overflow_rows(DiffLayout::Unified) {
        assert_eq!(
            &row[row.char_indices().nth(21).map_or(row.len(), |(i, _)| i)..],
            " ".repeat(20)
        );
    }
}

#[test]
fn side_by_side_text_stays_inside_its_half() {
    let rows = overflow_rows(DiffLayout::SideBySide);
    let blank = " ".repeat(20);
    assert_eq!(rows[1], format!("1   a very│1   a very{blank}"));
    assert_eq!(rows[2], format!("2 - old lo│2 + new lo{blank}"));
}
