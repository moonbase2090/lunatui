// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::{Cell, ClusterTable, Color, Grid, Rect, Style};
use lunatui_theme::{Theme, Variant};
use lunatui_widgets::{Breadcrumbs, RenderCx, Widget};

const PATH: [&str; 4] = ["Home", "Projects", "lunatui", "src"];

fn row(crumbs: &Breadcrumbs, width: u16) -> String {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(width, 1);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    crumbs.render(Rect::new(0, 0, width, 1), &mut cx);
    grid.row_display(0, &clusters)
}

fn shown(width: u16) -> String {
    row(&Breadcrumbs::new(PATH), width).trim_end().to_owned()
}

fn areas(crumbs: &Breadcrumbs, width: u16) -> Vec<(usize, Rect)> {
    crumbs.segment_areas(Rect::new(0, 0, width, 1))
}

#[test]
fn a_path_that_fits_is_drawn_in_full() {
    assert_eq!(shown(40), "Home › Projects › lunatui › src");
    assert_eq!(shown(31), "Home › Projects › lunatui › src");
}

#[test]
fn a_path_one_cell_too_wide_collapses_its_middle() {
    assert_eq!(shown(30), "Home › … › lunatui › src");
    assert_eq!(shown(24), "Home › … › lunatui › src");
}

#[test]
fn the_collapse_keeps_fewer_trailing_segments_as_the_room_shrinks() {
    assert_eq!(shown(23), "Home › … › src");
    assert_eq!(shown(14), "Home › … › src");
    assert_eq!(shown(13), "… › src");
    assert_eq!(shown(7), "… › src");
}

#[test]
fn the_last_segment_alone_is_cut_with_an_ellipsis_when_it_still_does_not_fit() {
    assert_eq!(shown(6), "src");
    let long = Breadcrumbs::new(["a", "long-directory-name"]);
    assert_eq!(row(&long, 8).trim_end(), "long-di…");
    assert_eq!(row(&long, 6).trim_end(), "long-…");
    assert_eq!(row(&long, 1), "…");
    let single = Breadcrumbs::new(["abcdefgh"]);
    assert_eq!(row(&single, 5), "abcd…");
    assert_eq!(row(&single, 8), "abcdefgh");
}

#[test]
fn two_segments_collapse_to_the_last() {
    let pair = Breadcrumbs::new(["usr", "local"]);
    assert_eq!(row(&pair, 11).trim_end(), "usr › local");
    assert_eq!(row(&pair, 10).trim_end(), "… › local");
    assert_eq!(row(&pair, 8).trim_end(), "local");
}

#[test]
fn one_segment_and_no_segments() {
    assert_eq!(row(&Breadcrumbs::new(["root"]), 8), "root    ");
    assert_eq!(row(&Breadcrumbs::new(Vec::<String>::new()), 4), "    ");
    assert_eq!(areas(&Breadcrumbs::new(Vec::<String>::new()), 4), []);
}

#[test]
fn a_custom_separator_is_used_everywhere() {
    let slashes = Breadcrumbs::new(PATH).separator("/");
    assert_eq!(row(&slashes, 30).trim_end(), "Home/Projects/lunatui/src");
    assert_eq!(row(&slashes, 20).trim_end(), "Home/…/lunatui/src");
    assert_eq!(row(&slashes, 12).trim_end(), "Home/…/src");
}

#[test]
fn wide_characters_count_by_display_width() {
    let wide = Breadcrumbs::new(["日本", "語"]);
    assert_eq!(row(&wide, 10).trim_end(), "日本 › 語");
    assert_eq!(row(&wide, 6).trim_end(), "… › 語");
    assert_eq!(row(&wide, 2).trim_end(), "語");
}

#[test]
fn segment_areas_follow_the_drawn_cells() {
    let crumbs = Breadcrumbs::new(PATH);
    assert_eq!(
        areas(&crumbs, 40),
        [
            (0, Rect::new(0, 0, 4, 1)),
            (1, Rect::new(7, 0, 8, 1)),
            (2, Rect::new(18, 0, 7, 1)),
            (3, Rect::new(28, 0, 3, 1)),
        ]
    );
    assert_eq!(
        crumbs.segment_areas(Rect::new(5, 3, 40, 2)),
        [
            (0, Rect::new(5, 3, 4, 1)),
            (1, Rect::new(12, 3, 8, 1)),
            (2, Rect::new(23, 3, 7, 1)),
            (3, Rect::new(33, 3, 3, 1)),
        ]
    );
}

#[test]
fn collapsed_segments_have_no_area_and_the_rest_keep_their_indexes() {
    let crumbs = Breadcrumbs::new(PATH);
    assert_eq!(
        areas(&crumbs, 30),
        [
            (0, Rect::new(0, 0, 4, 1)),
            (2, Rect::new(11, 0, 7, 1)),
            (3, Rect::new(21, 0, 3, 1)),
        ]
    );
    assert_eq!(
        areas(&crumbs, 20),
        [(0, Rect::new(0, 0, 4, 1)), (3, Rect::new(11, 0, 3, 1))]
    );
    assert_eq!(areas(&crumbs, 9), [(3, Rect::new(4, 0, 3, 1))]);
}

#[test]
fn a_cut_last_segment_covers_the_cells_that_show() {
    let long = Breadcrumbs::new(["a", "long-directory-name"]);
    assert_eq!(areas(&long, 6), [(1, Rect::new(0, 0, 6, 1))]);
    assert_eq!(areas(&long, 8), [(1, Rect::new(0, 0, 8, 1))]);
}

#[test]
fn empty_areas_draw_and_report_nothing() {
    let crumbs = Breadcrumbs::new(PATH);
    assert_eq!(crumbs.segment_areas(Rect::new(0, 0, 0, 1)), []);
    assert_eq!(crumbs.segment_areas(Rect::new(0, 0, 10, 0)), []);
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(10, 2);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    crumbs.render(Rect::new(0, 0, 0, 1), &mut cx);
    crumbs.render(Rect::new(0, 0, 10, 0), &mut cx);
    assert_eq!(grid.row_display(0, &clusters).trim(), "");
}

#[test]
fn styles_go_to_segments_the_last_segment_and_separators() {
    let red = Style::new().fg(Color::Rgb(255, 0, 0));
    let green = Style::new().fg(Color::Rgb(0, 255, 0));
    let blue = Style::new().fg(Color::Rgb(0, 0, 255));
    let crumbs = Breadcrumbs::new(["ab", "cd", "ef"])
        .style(red)
        .current_style(green)
        .separator_style(blue);
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(14, 1);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    crumbs.render(Rect::new(0, 0, 14, 1), &mut cx);
    let fg = |x| grid.get(x, 0).map(Cell::style).unwrap_or_default().fg;
    assert_eq!(fg(0), red.fg);
    assert_eq!(fg(1), red.fg);
    assert_eq!(fg(2), blue.fg);
    assert_eq!(fg(4), blue.fg);
    assert_eq!(fg(5), red.fg);
    assert_eq!(fg(7), blue.fg);
    assert_eq!(fg(10), green.fg);
    assert_eq!(fg(11), green.fg);
}

#[test]
fn the_collapsed_ellipsis_takes_the_segment_style() {
    let red = Style::new().fg(Color::Rgb(255, 0, 0));
    let crumbs = Breadcrumbs::new(PATH).style(red);
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(14, 1);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    crumbs.render(Rect::new(0, 0, 14, 1), &mut cx);
    assert_eq!(grid.row_display(0, &clusters).trim_end(), "Home › … › src");
    let style = grid.get(7, 0).map(Cell::style).unwrap_or_default();
    assert_eq!(style.fg, red.fg);
}

#[test]
fn hover_lays_its_style_over_one_segment_only() {
    let base = Style::new().fg(Color::Rgb(10, 10, 10));
    let hover = Style::new().fg(Color::Rgb(200, 0, 0));
    let crumbs = Breadcrumbs::new(["ab", "cd", "ef"])
        .style(base)
        .current_style(base)
        .hover(Some(1), hover);
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(14, 1);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    crumbs.render(Rect::new(0, 0, 14, 1), &mut cx);
    let fg = |x| grid.get(x, 0).map(Cell::style).unwrap_or_default().fg;
    assert_eq!(fg(0), base.fg);
    assert_eq!(fg(5), hover.fg);
    assert_eq!(fg(6), hover.fg);
    assert_eq!(fg(10), base.fg);
    assert_eq!(fg(11), base.fg);
}

#[test]
fn hovering_the_last_segment_and_a_collapsed_one() {
    let current = Style::new().fg(Color::Rgb(0, 200, 0));
    let hover = Style::new().fg(Color::Rgb(200, 0, 0));
    let theme = Theme::builtin(Variant::Dark);
    let on_last = Breadcrumbs::new(["ab", "cd"])
        .current_style(current)
        .hover(Some(1), hover);
    let mut grid = Grid::new(8, 1);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    on_last.render(Rect::new(0, 0, 8, 1), &mut cx);
    let fg = |grid: &Grid, x| grid.get(x, 0).map(Cell::style).unwrap_or_default().fg;
    assert_eq!(fg(&grid, 5), hover.fg);

    let hidden = Breadcrumbs::new(PATH).hover(Some(1), hover);
    let mut other = Grid::new(14, 1);
    let mut cx = RenderCx::new(&mut other, &mut clusters, &theme);
    hidden.render(Rect::new(0, 0, 14, 1), &mut cx);
    assert!((0..14).all(|x| fg(&other, x) != hover.fg));
}

#[test]
fn a_redraw_clears_what_was_in_the_row() {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(12, 1);
    let mut clusters = ClusterTable::new();
    {
        let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
        Breadcrumbs::new(["a", "bbbbbbbb"]).render(Rect::new(0, 0, 12, 1), &mut cx);
    }
    assert_eq!(grid.row_display(0, &clusters), "a › bbbbbbbb");
    {
        let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
        Breadcrumbs::new(["c"]).render(Rect::new(0, 0, 12, 1), &mut cx);
    }
    assert_eq!(grid.row_display(0, &clusters), "c           ");
}

#[test]
fn measure_wants_one_row() {
    let crumbs = Breadcrumbs::new(PATH);
    assert_eq!(
        crumbs.measure(lunatui_core::Size::new(40, 5)),
        lunatui_core::Size::new(40, 1)
    );
    assert_eq!(
        crumbs.measure(lunatui_core::Size::new(0, 5)),
        lunatui_core::Size::new(0, 0)
    );
}
