// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::{Attrs, ClusterTable, Color, Grid, Rect, Size, Style, UnderlineStyle};
use lunatui_text::markdown::{Options, parse};
use lunatui_theme::{Theme, Token, Variant};
use lunatui_widgets::{MarkdownView, RenderCx, Widget, WidgetState};

fn show(source: &str, width: u16, height: u16) -> Vec<String> {
    let theme = Theme::builtin(Variant::Dark);
    let document = parse(source, Options::gfm());
    let mut grid = Grid::new(width, height);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    MarkdownView::new(&document).render(Rect::new(0, 0, width, height), &mut cx);
    (0..height)
        .map(|row| grid.row_display(row, &clusters))
        .collect()
}

fn text_rows(source: &str, width: u16) -> Vec<String> {
    let document = parse(source, Options::gfm());
    let height = u16::try_from(MarkdownView::new(&document).content_height(width)).unwrap_or(0);
    show(source, width, height)
}

fn pad(rows: &[&str], width: usize) -> Vec<String> {
    rows.iter().map(|row| format!("{row:<width$}")).collect()
}

#[test]
fn paragraphs_wrap_at_word_boundaries() {
    assert_eq!(
        text_rows("one two three four five", 9),
        pad(&["one two", "three", "four five"], 9)
    );
    assert_eq!(text_rows("a\n\nb", 5), pad(&["a", "", "b"], 5));
}

#[test]
fn a_word_longer_than_the_width_is_split() {
    assert_eq!(
        text_rows("abcdefgh ij", 3),
        pad(&["abc", "def", "gh", "ij"], 3)
    );
}

#[test]
fn wide_characters_count_two_cells() {
    assert_eq!(text_rows("日本語 日本", 6), ["日本語", "日本  "]);
}

#[test]
fn hard_breaks_end_the_row_and_soft_breaks_are_spaces() {
    assert_eq!(text_rows("a\\\nb", 8), pad(&["a", "b"], 8));
    assert_eq!(text_rows("a\nb", 8), pad(&["a b"], 8));
}

#[test]
fn headings_one_and_two_get_a_rule() {
    assert_eq!(
        text_rows("# One\n\n## Two\n\n### Three", 8),
        pad(&["One", "════════", "", "Two", "────────", "", "Three"], 8)
    );
}

#[test]
fn thematic_breaks_span_the_width() {
    assert_eq!(
        text_rows("a\n\n---\n\nb", 4),
        pad(&["a", "", "────", "", "b"], 4)
    );
}

#[test]
fn bullets_change_with_depth() {
    assert_eq!(
        text_rows("- a\n  - b\n    - c\n      - d", 12),
        pad(&["• a", "  ◦ b", "    ▪ c", "      • d"], 12)
    );
}

#[test]
fn ordered_lists_align_their_numbers() {
    assert_eq!(
        text_rows("9. a\n10. b\n11. c", 8),
        pad(&[" 9. a", "10. b", "11. c"], 8)
    );
}

#[test]
fn task_items_show_their_state() {
    assert_eq!(
        text_rows("- [x] done\n- [ ] todo", 10),
        pad(&["☑ done", "☐ todo"], 10)
    );
}

#[test]
fn loose_lists_have_blank_rows_between_items() {
    assert_eq!(text_rows("- a\n\n- b", 4), pad(&["• a", "", "• b"], 4));
    assert_eq!(text_rows("- a\n- b", 4), pad(&["• a", "• b"], 4));
}

#[test]
fn list_items_wrap_with_a_hanging_indent() {
    assert_eq!(
        text_rows("- one two three", 8),
        pad(&["• one", "  two", "  three"], 8)
    );
}

#[test]
fn block_quotes_keep_their_bar_on_every_row() {
    assert_eq!(
        text_rows("> one two three\n>\n> four", 9),
        pad(&["▎ one two", "▎ three", "▎", "▎ four"], 9)
    );
    assert_eq!(text_rows("> > deep", 8), pad(&["▎ ▎ deep"], 8));
}

#[test]
fn code_blocks_are_padded_labeled_and_clipped() {
    assert_eq!(
        text_rows("```rust\nlet x = 1;\n\tlong line here\n```", 8),
        pad(
            &[
                "",
                "rust",
                "let x = 1;"[..8].to_owned().as_str(),
                "    long line here"[..8].to_owned().as_str(),
                ""
            ],
            8
        )
    );
}

#[test]
fn tables_are_boxed_and_aligned() {
    assert_eq!(
        text_rows(
            "| a | bb | c |\n|:--|:-:|--:|\n| 1 | 2 | 3 |\n| long | x | yy |",
            24
        ),
        pad(
            &[
                "┌──────┬────┬────┐",
                "│ a    │ bb │  c │",
                "├──────┼────┼────┤",
                "│ 1    │ 2  │  3 │",
                "│ long │ x  │ yy │",
                "└──────┴────┴────┘",
            ],
            24
        )
    );
}

#[test]
fn wide_tables_are_cut_with_an_ellipsis() {
    assert_eq!(
        text_rows("| head |\n|---|\n| abcdefghij |", 8),
        pad(
            &["┌──────┐", "│ head │", "├──────┤", "│ abc… │", "└──────┘"],
            8
        )
    );
}

#[test]
fn images_show_their_alt_text_and_html_is_kept() {
    assert_eq!(text_rows("![a logo](x.png)", 12), pad(&["[a logo]"], 12));
    assert_eq!(text_rows("<div>hi</div>", 14), pad(&["<div>hi</div>"], 14));
}

fn style_of(source: &str, x: u16, y: u16) -> lunatui_core::Style {
    let theme = Theme::builtin(Variant::Dark);
    let document = parse(source, Options::gfm());
    let mut grid = Grid::new(20, 8);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    MarkdownView::new(&document).render(Rect::new(0, 0, 20, 8), &mut cx);
    grid.get(x, y)
        .map(lunatui_core::Cell::style)
        .unwrap_or_default()
}

fn token(token: Token) -> Color {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(1, 1);
    let mut clusters = ClusterTable::new();
    RenderCx::new(&mut grid, &mut clusters, &theme).token(token)
}

#[test]
fn inline_marks_set_text_attributes() {
    let source = "**b** *i* ~~s~~ `c` [l](u) plain";
    let bold = style_of(source, 0, 0);
    assert!(bold.attrs.bold());
    assert!(!bold.attrs.italic());
    assert!(style_of(source, 2, 0).attrs.italic());
    assert!(style_of(source, 4, 0).attrs.strikethrough());
    let code = style_of(source, 6, 0);
    assert_eq!(code.bg, token(Token::Surface1));
    assert_eq!(code.fg, token(Token::Accent));
    let link = style_of(source, 8, 0);
    assert_eq!(link.fg, token(Token::FocusRing));
    assert_eq!(link.attrs.underline(), UnderlineStyle::Single);
    let plain = style_of(source, 10, 0);
    assert_eq!(plain.fg, token(Token::TextPrimary));
    assert_eq!(plain.attrs, Attrs::new());
}

#[test]
fn headings_are_bold_accent_and_quote_bars_are_accent() {
    let heading = style_of("# Title", 0, 0);
    assert!(heading.attrs.bold());
    assert_eq!(heading.fg, token(Token::Accent));
    let bar = style_of("> quote", 0, 0);
    assert_eq!(bar.fg, token(Token::Accent));
    let marker = style_of("- item", 0, 0);
    assert_eq!(marker.fg, token(Token::Accent));
    let rule = style_of("# T", 0, 1);
    assert!(rule.attrs.dim());
}

#[test]
fn code_rows_fill_the_whole_row_on_the_raised_surface() {
    for x in [0, 5, 19] {
        assert_eq!(
            style_of("```\nx\n```", x, 1).bg,
            token(Token::Surface1),
            "x={x}"
        );
    }
    assert_eq!(style_of("```\nx\n```", 0, 3).bg, Color::Default);
}

#[test]
fn background_styles_every_cell_and_disabled_dims() {
    let theme = Theme::builtin(Variant::Dark);
    let panel = Style::new().bg(Color::Rgb(10, 20, 30));
    let document = parse("# Hi\n\ntext", Options::gfm());
    let mut grid = Grid::new(10, 6);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    MarkdownView::new(&document)
        .background(panel)
        .state(WidgetState::Disabled)
        .render(Rect::new(0, 0, 10, 6), &mut cx);
    for y in 0..6 {
        for x in 0..10 {
            let style = grid
                .get(x, y)
                .map(lunatui_core::Cell::style)
                .unwrap_or_default();
            assert_eq!(style.bg, panel.bg, "bg at {x},{y}");
            assert!(style.attrs.dim(), "dim at {x},{y}");
        }
    }
}

#[test]
fn scrolling_moves_the_window_and_blank_rows_fill_the_end() {
    let document = parse("a\n\nb\n\nc", Options::gfm());
    let theme = Theme::builtin(Variant::Dark);
    let rows = |scroll| {
        let mut grid = Grid::new(3, 3);
        let mut clusters = ClusterTable::new();
        let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
        MarkdownView::new(&document)
            .scroll(scroll)
            .render(Rect::new(0, 0, 3, 3), &mut cx);
        (0..3)
            .map(|row| grid.row_display(row, &clusters))
            .collect::<Vec<_>>()
    };
    assert_eq!(rows(0), ["a  ", "   ", "b  "]);
    assert_eq!(rows(2), ["b  ", "   ", "c  "]);
    assert_eq!(rows(4), ["c  ", "   ", "   "]);
    assert_eq!(rows(99), ["   ", "   ", "   "]);
}

#[test]
fn content_height_and_measure_follow_the_width() {
    let document = parse("one two three four", Options::gfm());
    let view = MarkdownView::new(&document);
    assert_eq!(view.content_height(18), 1);
    assert_eq!(view.content_height(10), 2);
    assert_eq!(view.content_height(9), 3);
    assert_eq!(view.content_height(3), 6);
    assert_eq!(view.measure(Size::new(10, 10)), Size::new(10, 2));
    assert_eq!(view.measure(Size::new(3, 2)), Size::new(3, 2));
    assert_eq!(
        MarkdownView::new(&parse("", Options::gfm())).content_height(10),
        0
    );
}

#[test]
fn a_second_render_leaves_nothing_of_the_first() {
    let theme = Theme::builtin(Variant::Dark);
    let big = parse(
        "# Big\n\n> quote\n\n```\ncode\n```\n\n| a |\n|---|\n| b |",
        Options::gfm(),
    );
    let small = parse("x", Options::gfm());
    let cells = |grid: &Grid| -> Vec<u128> {
        (0..12)
            .flat_map(|y| (0..20).map(move |x| (x, y)))
            .map(|(x, y)| grid.cell_bits(x, y))
            .collect()
    };
    let mut grid = Grid::new(20, 12);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    MarkdownView::new(&big).render(Rect::new(0, 0, 20, 12), &mut cx);
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    MarkdownView::new(&small).render(Rect::new(0, 0, 20, 12), &mut cx);
    let mut fresh = Grid::new(20, 12);
    let mut fresh_clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut fresh, &mut fresh_clusters, &theme);
    MarkdownView::new(&small).render(Rect::new(0, 0, 20, 12), &mut cx);
    assert_eq!(cells(&grid), cells(&fresh));
}

#[test]
fn degenerate_areas_do_not_panic() {
    let source = "# T\n\n- a\n  > q\n\n```\nc\n```\n\n| a | b |\n|---|---|\n| 1 | 2 |";
    let document = parse(source, Options::gfm());
    let theme = Theme::builtin(Variant::Dark);
    for width in 0..7 {
        for height in 0..5 {
            let mut grid = Grid::new(width.max(1), height.max(1));
            let mut clusters = ClusterTable::new();
            let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
            MarkdownView::new(&document).render(Rect::new(0, 0, width, height), &mut cx);
        }
    }
}

const FRAGMENTS: &[&str] = &[
    "# ",
    "## ",
    "> ",
    "- ",
    "1. ",
    "- [x] ",
    "```",
    "~~~",
    "|",
    "---",
    "\n",
    "\n\n",
    "  ",
    "\t",
    "*",
    "**",
    "_",
    "~~",
    "`",
    "[",
    "](",
    ")",
    "![",
    "<",
    ">",
    "&amp;",
    "日本",
    "e\u{301}",
    "word",
    "a b c",
    "https://x.y",
    "<div>",
    "\\",
    "    ",
];

proptest::proptest! {
    #![proptest_config(proptest::prelude::ProptestConfig::with_cases(256))]

    #[test]
    fn any_markdown_renders_at_any_size_without_panicking(
        parts in proptest::collection::vec(proptest::sample::select(FRAGMENTS), 0..80),
        width in 0_u16..40,
        height in 0_u16..20,
        scroll in 0_usize..60,
    ) {
        let source: String = parts.concat();
        let document = parse(&source, Options::gfm());
        let theme = Theme::builtin(Variant::Dark);
        let mut grid = Grid::new(width.max(1), height.max(1));
        let mut clusters = ClusterTable::new();
        let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
        let view = MarkdownView::new(&document).scroll(scroll);
        view.render(Rect::new(0, 0, width, height), &mut cx);
        let rows = view.content_height(width);
        proptest::prop_assert!(view.measure(Size::new(width, height)).height <= height);
        proptest::prop_assert!(rows == 0 || width > 0);
    }
}

#[test]
fn nested_marks_keep_their_attribute() {
    assert!(style_of("*a *b* c*", 2, 0).attrs.italic());
    assert!(style_of("**a **b** c**", 2, 0).attrs.bold());
    assert!(style_of("~~a ~~b~~ c~~", 2, 0).attrs.strikethrough());
    let both = style_of("**a *b* c**", 2, 0);
    assert!(both.attrs.bold() && both.attrs.italic());
    assert!(style_of("# **b**", 0, 0).attrs.bold());
}

#[test]
fn heading_spaces_are_not_accented() {
    let space = style_of("# A B", 1, 0);
    assert!(!space.attrs.bold());
    assert_eq!(space.fg, token(Token::TextPrimary));
    assert!(style_of("# A B", 2, 0).attrs.bold());
}

#[test]
fn images_and_html_are_dim() {
    assert!(style_of("![alt](x)", 0, 0).attrs.dim());
    assert!(style_of("![alt](x)", 2, 0).attrs.dim());
    assert!(style_of("![alt](x)", 4, 0).attrs.dim());
    assert!(style_of("<b>x", 0, 0).attrs.dim());
    assert!(style_of("<div>\nx\n</div>", 0, 1).attrs.dim());
    assert!(!style_of("plain", 0, 0).attrs.dim());
}

#[test]
fn html_blocks_keep_their_lines() {
    assert_eq!(
        text_rows("<div>\nhi\n</div>", 8),
        pad(&["<div>", "hi", "</div>"], 8)
    );
}

#[test]
fn code_block_text_is_not_accented() {
    let code = style_of("```\nx\n```", 0, 1);
    assert_eq!(code.fg, token(Token::TextPrimary));
    assert_eq!(code.bg, token(Token::Surface1));
}

#[test]
fn nothing_is_drawn_beyond_the_area() {
    let theme = Theme::builtin(Variant::Dark);
    let document = parse("```\nvery long line of code here\n```", Options::gfm());
    let mut grid = Grid::new(20, 4);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    MarkdownView::new(&document).render(Rect::new(2, 0, 5, 4), &mut cx);
    assert_eq!(grid.row_display(1, &clusters), "  very              ");
    for x in 7..20 {
        let cell = grid
            .get(x, 1)
            .map(lunatui_core::Cell::style)
            .unwrap_or_default();
        assert_eq!(cell, Style::new(), "x={x}");
    }
}

#[test]
fn sibling_lists_start_at_the_top_level_glyph() {
    assert_eq!(
        text_rows("- a\n  - b\n\ntext\n\n- c", 8),
        pad(&["• a", "  ◦ b", "", "text", "", "• c"], 8)
    );
}

#[test]
fn centered_table_cells_split_the_padding() {
    assert_eq!(
        text_rows("| head |\n|:-:|\n| a |\n| bb |\n| ccc |", 12),
        pad(
            &[
                "┌──────┐",
                "│ head │",
                "├──────┤",
                "│  a   │",
                "│  bb  │",
                "│ ccc  │",
                "└──────┘",
            ],
            12
        )
    );
}

#[test]
fn leading_spaces_do_not_start_a_row() {
    assert_eq!(text_rows("`  a`", 8), pad(&["a"], 8));
}

#[test]
fn a_wide_character_in_one_column_gets_its_own_row() {
    assert_eq!(text_rows("日本", 1), [" ", " "]);
}

#[test]
fn rows_fill_to_the_last_cell_before_wrapping() {
    assert_eq!(text_rows("aa bb cc dd", 8), pad(&["aa bb cc", "dd"], 8));
    assert_eq!(text_rows("aa bb cc", 7), pad(&["aa bb", "cc"], 7));
    assert_eq!(text_rows("a**bc**", 3), pad(&["abc"], 3));
    assert_eq!(text_rows("ab cd ef", 5), pad(&["ab cd", "ef"], 5));
}
