// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::{Attrs, Cell, ClusterTable, Color, Grid, Rect, Size, Style};
use lunatui_theme::{Theme, Token, Variant};
use lunatui_widgets::{
    Marker, PieChart, PieLayout, PieSlice, RenderCx, Widget, WidgetState, blend_rgb,
};

const DARK: [(Token, (u8, u8, u8)); 8] = [
    (Token::Surface0, (0x1e, 0x1e, 0x2e)),
    (Token::Surface1, (0x31, 0x32, 0x44)),
    (Token::TextPrimary, (0xcd, 0xd6, 0xf4)),
    (Token::Accent, (0x9e, 0xce, 0x6a)),
    (Token::Ok, (0xa6, 0xe3, 0xa1)),
    (Token::Warn, (0xfa, 0xbd, 0x2f)),
    (Token::Fail, (0xf3, 0x8b, 0xa8)),
    (Token::FocusRing, (0xb4, 0xbc, 0xfa)),
];
const LIGHT: [(Token, (u8, u8, u8)); 8] = [
    (Token::Surface0, (0xf5, 0xf5, 0xf7)),
    (Token::Surface1, (0xe8, 0xe8, 0xed)),
    (Token::TextPrimary, (0x1a, 0x1a, 0x2e)),
    (Token::Accent, (0x7a, 0xa2, 0xf7)),
    (Token::Ok, (0x9e, 0xce, 0x6a)),
    (Token::Warn, (0xe6, 0x9f, 0x66)),
    (Token::Fail, (0xf3, 0x8b, 0xa8)),
    (Token::FocusRing, (0x89, 0xb4, 0xfa)),
];

fn themed(variant: Variant, colors: [(Token, (u8, u8, u8)); 8]) -> Theme {
    let mut theme = Theme::builtin(variant);
    for (token, (r, g, b)) in colors {
        theme.set_token(token, Color::Rgb(r, g, b));
    }
    theme
}

fn dark() -> Theme {
    themed(Variant::Dark, DARK)
}

fn light() -> Theme {
    themed(Variant::Light, LIGHT)
}

fn halves() -> PieChart {
    PieChart::new([PieSlice::new("A", 1.0), PieSlice::new("B", 1.0)]).legend(false)
}

fn full() -> PieChart {
    PieChart::new([PieSlice::new("A", 1.0)])
        .legend(false)
        .marker(Marker::Block)
}

fn draw(chart: &PieChart, width: u16, height: u16) -> (Grid, ClusterTable) {
    let theme = dark();
    let mut grid = Grid::new(width, height);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    chart.render(Rect::new(0, 0, width, height), &mut cx);
    (grid, clusters)
}

fn rows(chart: &PieChart, width: u16, height: u16) -> Vec<String> {
    let (grid, clusters) = draw(chart, width, height);
    (0..height)
        .map(|y| grid.row_display(y, &clusters))
        .collect()
}

fn text_at(grid: &Grid, clusters: &ClusterTable, x: u16, y: u16, len: usize) -> String {
    grid.row_display(y, clusters)
        .chars()
        .skip(usize::from(x))
        .take(len)
        .collect()
}

fn style_at(grid: &Grid, x: u16, y: u16) -> Style {
    grid.get(x, y).map(Cell::style).unwrap_or_default()
}

fn color(token: Token) -> Color {
    dark().resolve(token)
}

fn fg(token: Token) -> Style {
    Style::new().fg(color(token))
}

fn muted(token: Token) -> Color {
    let (Color::Rgb(r, g, b), Color::Rgb(sr, sg, sb)) = (color(token), color(Token::Surface0))
    else {
        return Color::Default;
    };
    let (r, g, b) = blend_rgb((r, g, b), (sr, sg, sb), 0.6);
    Color::Rgb(r, g, b)
}

#[test]
fn a_full_pie_fills_a_round_disc() {
    assert_eq!(rows(&full(), 6, 3), [" ████ ", "██████", " ████ "]);
    let (grid, _) = draw(&full(), 6, 3);
    assert_eq!(style_at(&grid, 1, 0), fg(Token::Accent));
    assert_eq!(style_at(&grid, 0, 0), Style::new());
}

#[test]
fn slices_run_clockwise_from_twelve_oclock() {
    let (grid, _) = draw(&halves().marker(Marker::Block), 6, 3);
    for (x, y) in [(3, 0), (4, 0), (5, 1), (4, 2), (3, 2)] {
        assert_eq!(style_at(&grid, x, y), fg(Token::Accent), "({x},{y})");
    }
    for (x, y) in [(1, 0), (2, 0), (0, 1), (1, 2), (2, 2)] {
        assert_eq!(style_at(&grid, x, y), fg(Token::FocusRing), "({x},{y})");
    }
}

#[test]
fn the_start_angle_turns_the_slices() {
    for degrees in [90.0, 450.0, -270.0] {
        let (grid, _) = draw(&halves().marker(Marker::Block).start_angle(degrees), 6, 3);
        for x in 1..5 {
            assert_eq!(style_at(&grid, x, 2), fg(Token::Accent), "{degrees} x={x}");
            assert_eq!(
                style_at(&grid, x, 0),
                fg(Token::FocusRing),
                "{degrees} x={x}"
            );
        }
    }
}

#[test]
fn a_slice_reaching_twelve_oclock_leaves_no_gap() {
    let chart = halves().marker(Marker::Braille);
    let (grid, clusters) = draw(&chart, 40, 20);
    for y in 0..20 {
        let left = grid.row_display(y, &clusters).chars().nth(19);
        let right = grid.row_display(y, &clusters).chars().nth(20);
        assert_eq!(
            left.is_some_and(|c| c != ' '),
            right.is_some_and(|c| c != ' '),
            "row {y}"
        );
    }
}

#[test]
fn half_blocks_put_two_slices_in_one_cell() {
    let chart = halves().start_angle(90.0);
    assert_eq!(rows(&chart, 6, 3), ["▄████▄", "▀▀▀▀▀▀", "▀████▀"]);
    let (grid, _) = draw(&chart, 6, 3);
    let ring = fg(Token::FocusRing);
    assert_eq!(style_at(&grid, 0, 0), ring);
    assert_eq!(style_at(&grid, 2, 0), ring);
    assert_eq!(style_at(&grid, 3, 1), ring.bg(color(Token::Accent)));
    assert_eq!(style_at(&grid, 0, 2), fg(Token::Accent));
}

#[test]
fn quadrants_draw_the_edge_in_quarter_cells() {
    let chart = halves().start_angle(90.0).marker(Marker::Quadrant);
    assert_eq!(rows(&chart, 6, 3), ["▗▟██▙▖", "▀▀▀▀▀▀", "▝▜██▛▘"]);
    let (grid, _) = draw(&chart, 6, 3);
    assert_eq!(
        style_at(&grid, 2, 1),
        fg(Token::FocusRing).bg(color(Token::Accent))
    );
    assert_eq!(style_at(&grid, 0, 0), fg(Token::FocusRing));
    assert_eq!(style_at(&grid, 0, 2), fg(Token::Accent));
}

#[test]
fn sextants_and_dots_use_their_own_glyphs() {
    let one = PieChart::new([PieSlice::new("A", 1.0)]).legend(false);
    assert_eq!(
        rows(&one.clone().marker(Marker::Sextant), 6, 3)[1],
        "██████"
    );
    assert_eq!(
        rows(&one.marker(Marker::Dot), 6, 3),
        [" •••• ", "••••••", " •••• "]
    );
}

#[test]
fn braille_draws_the_disc_in_dots() {
    let chart = PieChart::new([PieSlice::new("A", 1.0)])
        .legend(false)
        .marker(Marker::Braille);
    assert_eq!(rows(&chart, 2, 1), ["⢾⡷"]);
    let (grid, _) = draw(&chart, 2, 1);
    assert_eq!(style_at(&grid, 0, 0), fg(Token::Accent));
}

#[test]
fn braille_keeps_only_the_main_slice_dots_in_a_cell() {
    let chart = halves().marker(Marker::Braille);
    assert_eq!(rows(&chart, 2, 1), ["⢾⡷"]);
    let (grid, _) = draw(&chart, 2, 1);
    assert_eq!(style_at(&grid, 0, 0), fg(Token::FocusRing));
    assert_eq!(style_at(&grid, 1, 0), fg(Token::Accent));
    let split = halves().start_angle(90.0).marker(Marker::Braille);
    assert_eq!(rows(&split, 2, 1), ["⠚⠓"]);
    let (grid, _) = draw(&split, 2, 1);
    assert_eq!(style_at(&grid, 0, 0), fg(Token::FocusRing));
}

#[test]
fn a_donut_cuts_a_hole_for_the_center_text() {
    let chart = full().donut(0.5).center("7");
    assert_eq!(rows(&chart, 6, 3), [" ████ ", "██7 ██", " ████ "]);
    let (grid, _) = draw(&chart, 6, 3);
    assert_eq!(
        style_at(&grid, 2, 1),
        fg(Token::TextPrimary).with_attrs(Attrs::new().set_bold(true))
    );
}

#[test]
fn the_caption_sits_under_the_center_text_and_both_are_cut_to_the_hole() {
    let chart = full().donut(0.5).center("1,024,000 files").caption("total");
    let (grid, clusters) = draw(&chart, 20, 10);
    assert_eq!(text_at(&grid, &clusters, 6, 4, 7), "1,024,0");
    assert_eq!(text_at(&grid, &clusters, 13, 4, 1), " ");
    assert_eq!(text_at(&grid, &clusters, 7, 5, 5), "total");
    assert_eq!(
        style_at(&grid, 7, 5),
        fg(Token::TextPrimary).with_attrs(Attrs::new().set_dim(true))
    );
    let caption_only = full().donut(0.5).caption("total");
    let (grid, clusters) = draw(&caption_only, 20, 10);
    assert_eq!(text_at(&grid, &clusters, 7, 4, 5), "total");
}

#[test]
fn a_full_pie_shows_no_center_text() {
    assert_eq!(rows(&full().center("7"), 6, 3)[1], "██████");
}

#[test]
fn a_partial_reveal_leaves_a_track_for_the_rest() {
    let chart = full().reveal(0.5);
    let (grid, _) = draw(&chart, 6, 3);
    for (x, y) in [(3, 0), (5, 1), (3, 2)] {
        assert_eq!(style_at(&grid, x, y), fg(Token::Accent), "({x},{y})");
    }
    for (x, y) in [(2, 0), (0, 1), (2, 2)] {
        assert_eq!(style_at(&grid, x, y), fg(Token::Surface1), "({x},{y})");
    }
    let (grid, _) = draw(&chart.reveal(0.0), 6, 3);
    assert_eq!(style_at(&grid, 4, 0), fg(Token::Surface1));
}

#[test]
fn a_reveal_shrinks_every_slice_in_proportion() {
    let chart = halves().marker(Marker::Block).reveal(0.5);
    let (grid, _) = draw(&chart, 20, 10);
    assert_eq!(style_at(&grid, 15, 2), fg(Token::Accent));
    assert_eq!(style_at(&grid, 15, 7), fg(Token::FocusRing));
    assert_eq!(style_at(&grid, 4, 5), fg(Token::Surface1));
}

#[test]
fn nothing_is_drawn_for_an_empty_total() {
    let chart = PieChart::new([PieSlice::new("A", 0.0), PieSlice::new("B", -3.0)])
        .legend(false)
        .marker(Marker::Block);
    assert_eq!(rows(&chart, 6, 3), ["      "; 3]);
    let nan = PieChart::new([PieSlice::new("A", f64::NAN)]).legend(false);
    assert_eq!(nan.total(), 0.0);
    assert_eq!(PieSlice::new("B", -3.0).value(), 0.0);
    assert_eq!(PieSlice::new("C", 2.5).value(), 2.5);
}

#[test]
fn the_highlight_pulls_its_slice_out_from_the_center() {
    let plain = halves().marker(Marker::Block);
    let (grid, _) = draw(&plain, 20, 10);
    assert_eq!(style_at(&grid, 10, 4), fg(Token::Accent));
    assert_eq!(style_at(&grid, 0, 5), fg(Token::FocusRing));

    let pulled = plain.clone().highlight(Some(0));
    let (grid, clusters) = draw(&pulled, 20, 10);
    assert_eq!(text_at(&grid, &clusters, 10, 4, 1), " ");
    assert_eq!(text_at(&grid, &clusters, 0, 5, 1), " ");
    assert_eq!(style_at(&grid, 18, 5), fg(Token::Accent));
    assert_eq!(style_at(&grid, 8, 4), fg(Token::FocusRing));

    let out_of_range = plain.clone().highlight(Some(9));
    assert_eq!(rows(&out_of_range, 20, 10), rows(&plain, 20, 10));
}

#[test]
fn a_disabled_chart_mutes_the_slices_and_pulls_nothing() {
    let chart = halves()
        .marker(Marker::Block)
        .highlight(Some(0))
        .state(WidgetState::Disabled);
    let (grid, _) = draw(&chart, 20, 10);
    assert_ne!(muted(Token::Accent), color(Token::Accent));
    assert_eq!(
        style_at(&grid, 10, 4),
        Style::new().fg(muted(Token::Accent))
    );
    assert_eq!(
        style_at(&grid, 0, 5),
        Style::new().fg(muted(Token::FocusRing))
    );
}

fn mixed(a: Color, b: Color) -> Color {
    let (Color::Rgb(ar, ag, ab), Color::Rgb(br, bg, bb)) = (a, b) else {
        return Color::Default;
    };
    let (r, g, b) = blend_rgb((ar, ag, ab), (br, bg, bb), 0.5);
    Color::Rgb(r, g, b)
}

fn slice_colors(theme: &Theme, count: usize) -> Vec<Color> {
    let chart = PieChart::new((0..count).map(|n| PieSlice::new(format!("S{n}"), 1.0)));
    let mut grid = Grid::new(60, 20);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, theme);
    chart.render(Rect::new(0, 0, 60, 20), &mut cx);
    let legend = chart.layout(Rect::new(0, 0, 60, 20)).legend;
    (0..legend.height)
        .map(|row| style_at(&grid, legend.x, legend.y + row).fg)
        .collect()
}

#[test]
fn slice_colors_skip_tokens_too_close_to_ones_already_used() {
    let dark = dark();
    let pick = |token| dark.resolve(token);
    assert_eq!(
        slice_colors(&dark, 7),
        [
            pick(Token::Accent),
            pick(Token::FocusRing),
            pick(Token::Warn),
            pick(Token::Fail),
            mixed(pick(Token::Accent), pick(Token::Fail)),
            pick(Token::Accent),
            pick(Token::FocusRing),
        ]
    );
    let light = light();
    let pick = |token| light.resolve(token);
    assert_eq!(
        slice_colors(&light, 7),
        [
            pick(Token::Accent),
            pick(Token::Warn),
            pick(Token::Fail),
            pick(Token::Ok),
            mixed(pick(Token::Accent), pick(Token::Fail)),
            pick(Token::TextPrimary),
            mixed(pick(Token::Ok), pick(Token::FocusRing)),
        ]
    );
}

#[test]
fn the_last_slice_never_repeats_the_first_color() {
    let theme = dark();
    let pick = |token| theme.resolve(token);
    let six = slice_colors(&theme, 6);
    assert_eq!(six[5], pick(Token::FocusRing));
    assert_eq!(six[0], pick(Token::Accent));
    let eleven = slice_colors(&theme, 11);
    assert_eq!(eleven[5], pick(Token::Accent));
    assert_eq!(eleven[10], pick(Token::FocusRing));
    let mut two_colors = dark();
    let green = Color::Rgb(158, 206, 106);
    for token in [
        Token::Accent,
        Token::FocusRing,
        Token::Warn,
        Token::Ok,
        Token::TextPrimary,
    ] {
        two_colors.set_token(token, green);
    }
    two_colors.set_token(Token::Fail, Color::Rgb(158, 206, 206));
    assert_eq!(
        slice_colors(&two_colors, 3),
        [green, Color::Rgb(158, 206, 206), green]
    );
}

#[test]
fn indexed_tokens_are_used_when_they_differ() {
    let mut theme = dark();
    for (token, index) in [
        (Token::Accent, 2),
        (Token::FocusRing, 4),
        (Token::Warn, 3),
        (Token::Fail, 1),
        (Token::Ok, 2),
        (Token::TextPrimary, 7),
    ] {
        theme.set_token(token, Color::Indexed(index));
    }
    assert_eq!(
        slice_colors(&theme, 6),
        [2, 4, 3, 1, 7, 4].map(Color::Indexed)
    );
    for token in [
        Token::Accent,
        Token::FocusRing,
        Token::Warn,
        Token::Fail,
        Token::Ok,
        Token::TextPrimary,
    ] {
        theme.set_token(token, theme.resolve(Token::Surface0));
    }
    assert_eq!(slice_colors(&theme, 2), [theme.resolve(Token::Accent); 2]);
}

#[test]
fn a_slice_color_replaces_the_palette() {
    let custom = PieChart::new([PieSlice::new("A", 1.0).color(Color::Rgb(1, 2, 3))])
        .legend(false)
        .marker(Marker::Block);
    let (grid, _) = draw(&custom, 6, 3);
    assert_eq!(style_at(&grid, 2, 1), Style::new().fg(Color::Rgb(1, 2, 3)));
}

#[test]
fn text_keeps_the_background_under_the_chart() {
    let theme = dark();
    let panel = Color::Rgb(9, 9, 9);
    let mut grid = Grid::new(40, 10);
    let mut clusters = ClusterTable::new();
    grid.fill(
        Rect::new(0, 0, 40, 10),
        Style::new().bg(panel),
        ' ',
        &mut clusters,
    );
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    PieChart::new([PieSlice::new("Alpha", 1.0), PieSlice::new("Be", 1.0)])
        .donut(0.5)
        .center("7")
        .render(Rect::new(0, 0, 40, 10), &mut cx);
    assert_eq!(style_at(&grid, 24, 4).bg, panel);
    assert_eq!(style_at(&grid, 33, 4).bg, panel);
    assert_eq!(style_at(&grid, 22, 4).bg, panel);
    assert_eq!(style_at(&grid, 9, 4).bg, panel);
    assert_eq!(
        style_at(&grid, 9, 4),
        fg(Token::TextPrimary)
            .bg(panel)
            .with_attrs(Attrs::new().set_bold(true))
    );
    assert_eq!(style_at(&grid, 2, 4).bg, panel);
}

#[test]
fn colors_without_rgb_values_are_not_muted() {
    let chart = PieChart::new([PieSlice::new("A", 1.0).color(Color::Indexed(4))])
        .legend(false)
        .marker(Marker::Block)
        .state(WidgetState::Disabled);
    let (grid, _) = draw(&chart, 6, 3);
    assert_eq!(style_at(&grid, 2, 1), Style::new().fg(Color::Indexed(4)));
}

#[test]
fn the_legend_goes_beside_the_disc_when_that_leaves_more_room() {
    let chart = PieChart::new([PieSlice::new("A", 1.0), PieSlice::new("Bb", 1.0)]);
    assert_eq!(
        chart.layout(Rect::new(0, 0, 40, 10)),
        PieLayout {
            pie: Rect::new(0, 0, 20, 10),
            legend: Rect::new(22, 4, 9, 2),
        }
    );
    assert_eq!(
        chart.layout(Rect::new(3, 2, 40, 10)),
        PieLayout {
            pie: Rect::new(3, 2, 20, 10),
            legend: Rect::new(25, 6, 9, 2),
        }
    );
}

#[test]
fn the_legend_goes_under_the_disc_when_that_leaves_more_room() {
    let chart = PieChart::new([PieSlice::new("A", 1.0), PieSlice::new("Bb", 1.0)]);
    assert_eq!(
        chart.layout(Rect::new(0, 0, 30, 20)),
        PieLayout {
            pie: Rect::new(0, 0, 30, 15),
            legend: Rect::new(10, 16, 9, 2),
        }
    );
}

#[test]
fn without_a_legend_the_disc_is_centered() {
    let chart = halves();
    assert_eq!(
        chart.layout(Rect::new(0, 0, 40, 10)),
        PieLayout {
            pie: Rect::new(10, 0, 20, 10),
            legend: Rect::new(0, 0, 0, 0),
        }
    );
    let empty = PieChart::new(Vec::<PieSlice>::new());
    assert_eq!(
        empty.layout(Rect::new(0, 0, 40, 10)).pie,
        Rect::new(10, 0, 20, 10)
    );
}

#[test]
fn the_cell_aspect_keeps_the_disc_round_and_is_clamped() {
    let chart = halves();
    let area = Rect::new(0, 0, 40, 10);
    assert_eq!(
        chart.clone().cell_aspect(1.0).layout(area).pie,
        Rect::new(15, 0, 10, 10)
    );
    assert_eq!(
        chart.clone().cell_aspect(9.0).layout(area),
        chart.clone().cell_aspect(4.0).layout(area)
    );
    assert_eq!(
        chart.clone().cell_aspect(0.1).layout(area).pie,
        Rect::new(17, 0, 5, 10)
    );
    assert_eq!(
        chart.clone().cell_aspect(f64::NAN).layout(area),
        chart.layout(area)
    );
}

#[test]
fn the_legend_lists_each_slice_with_its_share() {
    let chart = PieChart::new([PieSlice::new("Alpha", 1.0), PieSlice::new("Be", 1.0)]);
    let (grid, clusters) = draw(&chart, 40, 10);
    assert_eq!(
        chart.layout(Rect::new(0, 0, 40, 10)).legend,
        Rect::new(22, 4, 12, 2)
    );
    assert_eq!(text_at(&grid, &clusters, 22, 4, 12), "● Alpha  50%");
    assert_eq!(text_at(&grid, &clusters, 22, 5, 12), "● Be     50%");
    assert_eq!(style_at(&grid, 22, 4), fg(Token::Accent));
    assert_eq!(style_at(&grid, 22, 5), fg(Token::FocusRing));
    assert_eq!(style_at(&grid, 24, 4), fg(Token::TextPrimary));
    assert_eq!(
        style_at(&grid, 33, 4),
        fg(Token::TextPrimary).with_attrs(Attrs::new().set_dim(true))
    );
}

#[test]
fn the_highlighted_legend_row_is_bold_and_marked_when_focused() {
    let chart =
        PieChart::new([PieSlice::new("Alpha", 1.0), PieSlice::new("Be", 1.0)]).highlight(Some(1));
    let bold = Attrs::new().set_bold(true);
    let (grid, _) = draw(&chart, 40, 10);
    assert_eq!(
        style_at(&grid, 24, 5),
        fg(Token::TextPrimary).with_attrs(bold)
    );
    assert_eq!(
        style_at(&grid, 33, 5),
        fg(Token::TextPrimary).with_attrs(bold)
    );
    assert_eq!(style_at(&grid, 24, 4), fg(Token::TextPrimary));

    let (grid, _) = draw(&chart.clone().state(WidgetState::Focused), 40, 10);
    assert_eq!(
        style_at(&grid, 24, 5),
        fg(Token::FocusRing).with_attrs(bold)
    );
    assert_eq!(style_at(&grid, 24, 4), fg(Token::TextPrimary));

    let (grid, _) = draw(&chart.state(WidgetState::Disabled), 40, 10);
    let dim = Attrs::new().set_dim(true);
    assert_eq!(
        style_at(&grid, 24, 5),
        fg(Token::TextPrimary).with_attrs(dim)
    );
    assert_eq!(
        style_at(&grid, 33, 5),
        fg(Token::TextPrimary).with_attrs(dim)
    );
    assert_eq!(
        style_at(&grid, 22, 4),
        Style::new().fg(muted(Token::Accent))
    );
}

#[test]
fn long_labels_are_cut_and_rows_stop_at_the_area() {
    let chart = PieChart::new((0..5).map(|n| PieSlice::new(format!("Slice {n}"), 1.0)));
    let layout = chart.layout(Rect::new(0, 0, 40, 3));
    assert_eq!(layout.legend, Rect::new(8, 0, 14, 3));
    let (grid, clusters) = draw(&chart, 40, 3);
    assert_eq!(text_at(&grid, &clusters, 8, 2, 14), "● Slice 2  20%");

    let narrow = PieChart::new([PieSlice::new("A very long label", 1.0)]);
    let layout = narrow.layout(Rect::new(0, 0, 12, 20));
    assert_eq!(layout.legend, Rect::new(0, 7, 12, 1));
    let (grid, clusters) = draw(&narrow, 12, 20);
    assert_eq!(grid.row_display(7, &clusters), "● A ver 100%");
}

#[test]
fn percents_add_up_to_one_hundred() {
    let thirds = PieChart::new((0..3).map(|n| PieSlice::new(format!("{n}"), 1.0)));
    assert_eq!(thirds.percents(), [34, 33, 33]);
    let uneven = PieChart::new([PieSlice::new("a", 1.0), PieSlice::new("b", 2.0)]);
    assert_eq!(uneven.percents(), [33, 67]);
    let exact = PieChart::new([
        PieSlice::new("a", 45.0),
        PieSlice::new("b", 25.0),
        PieSlice::new("c", 18.0),
        PieSlice::new("d", 12.0),
    ]);
    assert_eq!(exact.percents(), [45, 25, 18, 12]);
    assert_eq!(exact.total(), 100.0);
    let small = PieChart::new([
        PieSlice::new("a", 1.0),
        PieSlice::new("b", 1.0),
        PieSlice::new("c", 1.0),
        PieSlice::new("d", 97.4),
    ]);
    assert_eq!(small.percents(), [1, 1, 1, 97]);
    let empty = PieChart::new([PieSlice::new("a", 0.0), PieSlice::new("b", 0.0)]);
    assert_eq!(empty.percents(), [0, 0]);
}

#[test]
fn slice_at_finds_the_slice_under_a_cell() {
    let chart = halves().marker(Marker::Block);
    let area = Rect::new(0, 0, 6, 3);
    assert_eq!(chart.slice_at(area, 4, 0), Some(0));
    assert_eq!(chart.slice_at(area, 1, 1), Some(1));
    assert_eq!(chart.slice_at(area, 0, 0), None);
    assert_eq!(chart.slice_at(area, 6, 1), None);
    assert_eq!(chart.slice_at(area, 2, 3), None);
    let offset = Rect::new(10, 5, 6, 3);
    assert_eq!(chart.slice_at(offset, 14, 5), Some(0));
    assert_eq!(chart.slice_at(offset, 9, 6), None);
    assert_eq!(chart.slice_at(offset, 11, 4), None);
    let track = full().reveal(0.5);
    assert_eq!(track.slice_at(area, 1, 1), None);
    assert_eq!(track.slice_at(area, 4, 1), Some(0));
    let empty = PieChart::new([PieSlice::new("A", 0.0)]).legend(false);
    assert_eq!(empty.slice_at(area, 2, 1), None);
}

#[test]
fn measure_covers_the_disc_and_the_legend() {
    let chart = PieChart::new([PieSlice::new("A", 1.0), PieSlice::new("Bb", 1.0)]);
    assert_eq!(chart.measure(Size::new(40, 10)), Size::new(31, 10));
    assert_eq!(halves().measure(Size::new(40, 10)), Size::new(30, 10));
    assert_eq!(chart.measure(Size::new(30, 20)), Size::new(30, 18));
}

#[test]
fn settings_clamp_their_inputs() {
    assert_eq!(
        rows(&full().donut(f64::NAN), 6, 3),
        rows(&full().donut(0.0), 6, 3)
    );
    assert_eq!(
        rows(&full().donut(5.0), 20, 10),
        rows(&full().donut(0.9), 20, 10)
    );
    assert_ne!(
        rows(&full().donut(0.9), 20, 10),
        rows(&full().donut(0.5), 20, 10)
    );
    assert_eq!(
        rows(&full().reveal(f64::NAN), 6, 3),
        rows(&full().reveal(0.0), 6, 3)
    );
    assert_eq!(rows(&full().reveal(3.0), 6, 3), rows(&full(), 6, 3));
    assert_eq!(
        rows(&halves().start_angle(f64::INFINITY), 6, 3),
        rows(&halves(), 6, 3)
    );
}

#[cfg(feature = "color-profiles")]
#[test]
fn a_sixteen_color_palette_still_gives_neighbouring_slices_different_colors() {
    use lunatui_theme::Profile;
    for variant in [Variant::Dark, Variant::Light] {
        let theme = Theme::builtin(variant);
        let palette = theme.prepare(Profile::Ansi16);
        let chart = PieChart::new((0..4).map(|n| PieSlice::new(format!("S{n}"), 1.0)));
        let mut grid = Grid::new(60, 20);
        let mut clusters = ClusterTable::new();
        let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme).palette(&palette);
        chart.render(Rect::new(0, 0, 60, 20), &mut cx);
        let legend = chart.layout(Rect::new(0, 0, 60, 20)).legend;
        let colors: Vec<Color> = (0..legend.height)
            .map(|row| style_at(&grid, legend.x, legend.y + row).fg)
            .collect();
        for (index, color) in colors.iter().enumerate() {
            assert!(
                matches!(color, Color::Indexed(n) if *n < 16),
                "{variant:?} {color:?}"
            );
            let next = colors[(index + 1) % colors.len()];
            assert_ne!(*color, next, "{variant:?} slices {index} and its neighbour");
        }
        let unique: std::collections::BTreeSet<_> =
            colors.iter().map(|color| format!("{color:?}")).collect();
        assert_eq!(unique.len(), 4, "{variant:?} {colors:?}");
    }
}

#[test]
fn an_offset_area_with_room_to_spare_centers_the_disc_and_legend() {
    let chart = PieChart::new([PieSlice::new("A", 1.0), PieSlice::new("Bb", 1.0)]);
    assert_eq!(
        chart.layout(Rect::new(3, 2, 30, 12)),
        PieLayout {
            pie: Rect::new(3, 3, 19, 9),
            legend: Rect::new(24, 7, 9, 2),
        }
    );
    assert_eq!(
        chart.cell_aspect(1.0).layout(Rect::new(5, 4, 30, 30)),
        PieLayout {
            pie: Rect::new(6, 4, 27, 27),
            legend: Rect::new(15, 32, 9, 2),
        }
    );
    assert_eq!(
        halves().layout(Rect::new(3, 2, 24, 20)).pie,
        Rect::new(3, 6, 24, 12)
    );
    assert_eq!(
        halves().layout(Rect::new(3, 2, 40, 12)).pie,
        Rect::new(11, 2, 24, 12)
    );
}

#[test]
fn slice_at_reads_the_middle_of_the_cell() {
    let chart = halves().marker(Marker::Block);
    assert_eq!(chart.slice_at(Rect::new(10, 5, 6, 3), 10, 6), Some(1));
    let split = halves().start_angle(90.0);
    let area = Rect::new(0, 0, 20, 10);
    assert_eq!(split.slice_at(area, 10, 5), Some(0));
    assert_eq!(split.slice_at(area, 10, 4), Some(1));
}

#[test]
fn the_pull_follows_the_middle_of_a_partly_revealed_slice() {
    let chart = halves()
        .marker(Marker::Block)
        .reveal(0.5)
        .highlight(Some(0));
    let (grid, clusters) = draw(&chart, 20, 10);
    assert_eq!(style_at(&grid, 13, 0), fg(Token::Accent));
    assert_eq!(style_at(&grid, 16, 1), fg(Token::Accent));
    assert_eq!(text_at(&grid, &clusters, 17, 1, 1), " ");
    assert_eq!(text_at(&grid, &clusters, 10, 4, 1), " ");
}

#[test]
fn a_highlight_just_past_the_last_slice_pulls_nothing() {
    let plain = halves().marker(Marker::Block);
    assert_eq!(
        rows(&plain.clone().highlight(Some(2)), 20, 10),
        rows(&plain, 20, 10)
    );
}

#[test]
fn a_sextant_edge_cell_shows_its_part_of_the_disc() {
    let chart = halves().start_angle(90.0).marker(Marker::Sextant);
    let first = rows(&chart, 6, 3)[0].chars().next();
    assert_eq!(first, char::from_u32(0x1FB1E));
}

#[test]
fn a_cell_takes_the_slice_covering_most_of_it() {
    let chart = halves().start_angle(78.5).marker(Marker::Quadrant);
    let (grid, clusters) = draw(&chart, 6, 3);
    assert_eq!(text_at(&grid, &clusters, 0, 1, 1), "▛");
    assert_eq!(
        style_at(&grid, 0, 1),
        fg(Token::FocusRing).bg(color(Token::Accent))
    );
}

#[test]
fn spare_room_is_split_evenly_around_the_disc() {
    let five = PieChart::new(["A", "B", "C", "D", "E"].map(|label| PieSlice::new(label, 1.0)));
    assert_eq!(
        five.layout(Rect::new(3, 2, 30, 14)),
        PieLayout {
            pie: Rect::new(3, 4, 20, 10),
            legend: Rect::new(25, 6, 8, 5),
        }
    );
    let two = PieChart::new([PieSlice::new("A", 1.0), PieSlice::new("Bb", 1.0)]).cell_aspect(1.0);
    assert_eq!(
        two.layout(Rect::new(5, 4, 31, 30)),
        PieLayout {
            pie: Rect::new(7, 4, 27, 27),
            legend: Rect::new(16, 32, 9, 2),
        }
    );
}

#[test]
fn a_tiny_area_keeps_what_fits_of_the_legend() {
    let chart = PieChart::new([PieSlice::new("A", 1.0), PieSlice::new("Bb", 1.0)]);
    assert_eq!(
        chart.layout(Rect::new(0, 0, 5, 1)),
        PieLayout {
            pie: Rect::new(0, 0, 0, 0),
            legend: Rect::new(2, 0, 3, 1),
        }
    );
    assert_eq!(rows(&chart, 5, 1), ["  ●  "]);
}

#[test]
fn center_text_follows_an_offset_area() {
    let chart = full().donut(0.5).center("7");
    let theme = dark();
    let mut grid = Grid::new(12, 5);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    chart.render(Rect::new(4, 1, 6, 3), &mut cx);
    assert_eq!(text_at(&grid, &clusters, 4, 2, 6), "██7 ██");
}
