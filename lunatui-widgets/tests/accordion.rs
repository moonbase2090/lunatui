// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::{Cell, ClusterTable, Color, Grid, Rect, Size, Style};
use lunatui_theme::{Theme, Token, Variant};
use lunatui_widgets::{Accordion, AccordionSection, RenderCx, SectionArea, Widget, WidgetState};

fn sections() -> Vec<AccordionSection> {
    vec![
        AccordionSection::new("Alpha", 2).open(true),
        AccordionSection::new("Beta", 3),
        AccordionSection::new("Gamma", 1).open(true),
    ]
}

fn accordion() -> Accordion {
    Accordion::new(sections())
}

fn draw(accordion: &Accordion, area: Rect, width: u16, height: u16) -> (Grid, ClusterTable) {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(width, height);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    accordion.render(area, &mut cx);
    (grid, clusters)
}

fn rows(accordion: &Accordion, width: u16, height: u16) -> Vec<String> {
    let (grid, clusters) = draw(accordion, Rect::new(0, 0, width, height), width, height);
    (0..height)
        .map(|y| grid.row_display(y, &clusters))
        .collect()
}

fn style_at(grid: &Grid, x: u16, y: u16) -> Style {
    grid.get(x, y).map(Cell::style).unwrap_or_default()
}

fn color(token: Token) -> Color {
    Theme::builtin(Variant::Dark).resolve(token)
}

fn parts(area: &SectionArea) -> [Rect; 2] {
    [area.header, area.body]
}

#[test]
fn open_sections_get_body_rows_under_their_headers() {
    let areas = accordion().layout(Rect::new(0, 0, 20, 12));
    assert_eq!(
        parts(&areas[0]),
        [Rect::new(0, 0, 20, 1), Rect::new(2, 1, 18, 2)]
    );
    assert_eq!(
        parts(&areas[1]),
        [Rect::new(0, 3, 20, 1), Rect::new(2, 4, 18, 0)]
    );
    assert_eq!(
        parts(&areas[2]),
        [Rect::new(0, 4, 20, 1), Rect::new(2, 5, 18, 1)]
    );
    assert_eq!(areas.iter().map(|a| a.index).collect::<Vec<_>>(), [0, 1, 2]);
}

#[test]
fn the_area_offset_and_gap_move_the_sections() {
    let areas = accordion().gap(1).layout(Rect::new(3, 2, 20, 12));
    assert_eq!(parts(&areas[0])[0], Rect::new(3, 2, 20, 1));
    assert_eq!(parts(&areas[0])[1], Rect::new(5, 3, 18, 2));
    assert_eq!(parts(&areas[1])[0].y, 6);
    assert_eq!(parts(&areas[2])[0].y, 8);
    assert_eq!(parts(&areas[2])[1], Rect::new(5, 9, 18, 1));
}

#[test]
fn a_reveal_fraction_shows_that_share_of_the_body_rows() {
    let rows_at = |reveal: f64| {
        Accordion::new([AccordionSection::new("S", 4).reveal(reveal)])
            .layout(Rect::new(0, 0, 10, 12))[0]
            .body
            .height
    };
    assert_eq!(rows_at(0.0), 0);
    assert_eq!(rows_at(0.3), 1);
    assert_eq!(rows_at(0.6), 2);
    assert_eq!(rows_at(0.8), 3);
    assert_eq!(rows_at(1.0), 4);
    assert_eq!(rows_at(9.0), 4);
    assert_eq!(rows_at(-2.0), 0);
    assert_eq!(rows_at(f64::NAN), 0);
}

#[test]
fn a_reveal_leaves_the_caret_to_the_open_flag() {
    let sections = [
        AccordionSection::new("Closing", 4).open(false).reveal(0.5),
        AccordionSection::new("Opening", 4).open(true).reveal(0.25),
    ];
    let rendered = rows(&Accordion::new(sections), 12, 12);
    assert_eq!(rendered[0], "▸ Closing   ");
    assert_eq!(rendered[3], "▾ Opening   ");
}

#[test]
fn a_zero_row_section_has_no_body() {
    let none = Accordion::new([AccordionSection::new("S", 0).open(true)]);
    assert_eq!(none.layout(Rect::new(0, 0, 10, 5))[0].body.height, 0);
}

#[test]
fn bodies_are_cut_to_the_rows_that_remain() {
    let areas = accordion().layout(Rect::new(0, 0, 20, 2));
    assert_eq!(areas.len(), 1);
    assert_eq!(parts(&areas[0])[1].height, 1);
    let tall = Accordion::new([AccordionSection::new("S", 9).open(true)]);
    assert_eq!(tall.layout(Rect::new(0, 0, 10, 4))[0].body.height, 3);
    assert_eq!(tall.layout(Rect::new(0, 2, 10, 4))[0].body.height, 3);
}

#[test]
fn sections_below_the_area_and_empty_areas_give_nothing() {
    assert_eq!(accordion().layout(Rect::new(0, 0, 20, 5)).len(), 3);
    assert_eq!(accordion().layout(Rect::new(0, 0, 20, 4)).len(), 2);
    assert_eq!(accordion().layout(Rect::new(0, 0, 20, 3)).len(), 1);
    assert_eq!(accordion().layout(Rect::new(0, 0, 20, 0)), []);
    assert_eq!(accordion().layout(Rect::new(0, 0, 0, 5)), []);
    assert_eq!(
        Accordion::new(Vec::<AccordionSection>::new()).layout(Rect::new(0, 0, 9, 9)),
        []
    );
}

#[test]
fn narrow_areas_shrink_the_indent() {
    let area = accordion().layout(Rect::new(0, 0, 1, 5));
    assert_eq!(area[0].body, Rect::new(1, 1, 0, 2));
    let two = accordion().layout(Rect::new(0, 0, 2, 5));
    assert_eq!(two[0].body, Rect::new(2, 1, 0, 2));
}

#[test]
fn headers_show_a_caret_the_title_and_a_right_aligned_badge() {
    let sections = [
        AccordionSection::new("Alpha", 1).open(true).badge("3"),
        AccordionSection::new("Beta", 1).badge("12 files"),
    ];
    assert_eq!(
        rows(&Accordion::new(sections), 20, 4),
        [
            "▾ Alpha            3",
            "                    ",
            "▸ Beta      12 files",
            "                    ",
        ]
    );
}

#[test]
fn long_titles_are_clipped_before_the_badge_and_the_badge_drops_when_it_cannot_fit() {
    let sections = [AccordionSection::new("Dependencies", 1).badge("42")];
    assert_eq!(
        rows(&Accordion::new(sections.clone()), 10, 1),
        ["▸ Depen 42"]
    );
    assert_eq!(rows(&Accordion::new(sections), 3, 1), ["▸ D"]);
    let tight = [AccordionSection::new("Ab", 1).badge("123456")];
    assert_eq!(rows(&Accordion::new(tight), 9, 1), ["▸ Ab     "]);
}

#[test]
fn colors_follow_focus_and_the_theme() {
    let sections = [
        AccordionSection::new("Alpha", 1).badge("3"),
        AccordionSection::new("Beta", 1),
    ];
    let focused = Accordion::new(sections.clone()).focus(Some(1));
    let (grid, _) = draw(&focused, Rect::new(0, 0, 12, 4), 12, 4);
    assert_eq!(style_at(&grid, 0, 0).fg, color(Token::TextPrimary));
    assert!(!style_at(&grid, 0, 0).attrs.bold());
    assert_eq!(style_at(&grid, 2, 0).fg, color(Token::TextPrimary));
    assert!(style_at(&grid, 11, 0).attrs.dim());
    assert_eq!(style_at(&grid, 0, 1).fg, color(Token::Accent));
    assert!(style_at(&grid, 0, 1).attrs.bold());
    assert!(style_at(&grid, 4, 1).attrs.bold());
}

#[test]
fn disabled_sections_and_accordions_are_dimmed_and_never_highlighted() {
    let sections = [
        AccordionSection::new("Alpha", 1).disabled(),
        AccordionSection::new("Beta", 1),
    ];
    let one = Accordion::new(sections.clone()).focus(Some(0));
    let (grid, _) = draw(&one, Rect::new(0, 0, 12, 4), 12, 4);
    assert!(style_at(&grid, 2, 0).attrs.dim());
    assert!(!style_at(&grid, 2, 0).attrs.bold());
    assert_eq!(style_at(&grid, 2, 0).fg, color(Token::TextPrimary));
    assert!(!style_at(&grid, 2, 1).attrs.dim());
    let all = Accordion::new(sections).state(WidgetState::Disabled);
    let (grid, _) = draw(&all, Rect::new(0, 0, 12, 4), 12, 4);
    assert!(style_at(&grid, 2, 0).attrs.dim());
    assert!(style_at(&grid, 2, 1).attrs.dim());
}

#[test]
fn an_explicit_style_replaces_the_theme_colors() {
    let style = Style::new().fg(Color::Rgb(1, 2, 3));
    let sections = [AccordionSection::new("Alpha", 1).badge("3")];
    let (grid, _) = draw(
        &Accordion::new(sections).style(style),
        Rect::new(0, 0, 12, 2),
        12,
        2,
    );
    assert_eq!(style_at(&grid, 0, 0), style);
    assert_eq!(style_at(&grid, 11, 0), style);
}

#[test]
fn step_moves_through_the_headers_wrapping_and_skipping_disabled_sections() {
    let accordion = Accordion::new([
        AccordionSection::new("A", 1),
        AccordionSection::new("B", 1).disabled(),
        AccordionSection::new("C", 1),
    ]);
    assert_eq!(accordion.step(None, true), Some(0));
    assert_eq!(accordion.step(None, false), Some(2));
    assert_eq!(accordion.step(Some(0), true), Some(2));
    assert_eq!(accordion.step(Some(2), true), Some(0));
    assert_eq!(accordion.step(Some(2), false), Some(0));
    assert_eq!(accordion.step(Some(0), false), Some(2));
    assert_eq!(accordion.step(Some(3), true), Some(0));
    assert_eq!(accordion.step(Some(3), false), Some(2));
    let none = Accordion::new([AccordionSection::new("A", 1).disabled()]);
    assert_eq!(none.step(None, true), None);
    assert_eq!(
        Accordion::new(Vec::<AccordionSection>::new()).step(None, true),
        None
    );
}

#[test]
fn len_and_is_empty_count_the_sections() {
    assert_eq!(accordion().len(), 3);
    assert!(!accordion().is_empty());
    assert!(Accordion::new(Vec::<AccordionSection>::new()).is_empty());
}

#[test]
fn measure_is_the_rows_the_headers_and_open_bodies_use() {
    assert_eq!(accordion().measure(Size::new(20, 12)), Size::new(20, 6));
    assert_eq!(
        accordion().gap(1).measure(Size::new(20, 12)),
        Size::new(20, 8)
    );
    assert_eq!(accordion().measure(Size::new(20, 2)), Size::new(20, 2));
    assert_eq!(accordion().measure(Size::new(0, 5)), Size::new(0, 0));
    assert_eq!(accordion().measure(Size::new(5, 0)), Size::new(0, 0));
}

#[test]
fn nothing_is_drawn_outside_the_area() {
    let sections = [AccordionSection::new("Alpha", 1).open(true)];
    let (grid, clusters) = draw(&Accordion::new(sections), Rect::new(2, 1, 6, 2), 12, 5);
    assert_eq!(grid.row_display(0, &clusters).trim(), "");
    let title = "Alpha";
    assert_eq!(
        grid.row_display(1, &clusters),
        format!("  ▾ {}    ", &title[..4])
    );
    assert_eq!(grid.row_display(3, &clusters).trim(), "");
}
