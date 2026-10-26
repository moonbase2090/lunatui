// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::{ClusterTable, Color, DecodedGlyph, Grid, Rect, Style};
use lunatui_theme::{Theme, Token, Variant};
use lunatui_widgets::{LineGauge, LineSet, RenderCx, Widget, WidgetState};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect as RatRect;
use ratatui::style::{Color as RatColor, Style as RatStyle};
use ratatui::widgets::{LineGauge as RatGauge, Widget as RatWidget};

fn render(gauge: &LineGauge<'_>, width: u16, height: u16) -> (Grid, ClusterTable) {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(width, height);
    let mut clusters = ClusterTable::new();
    gauge.render(
        Rect::new(0, 0, width, height),
        &mut RenderCx::new(&mut grid, &mut clusters, &theme),
    );
    (grid, clusters)
}

#[test]
fn literal_progress_and_symbol_validation() {
    let gauge = LineGauge::new()
        .value_label("")
        .subcell(false)
        .label("")
        .ratio(0.5)
        .filled_symbol("+")
        .unfilled_symbol("-");
    let (grid, clusters) = render(&gauge, 7, 1);
    assert_eq!(grid.row_display(0, &clusters), " +++---");
    let (grid, clusters) = render(
        &LineGauge::new()
            .value_label("")
            .subcell(false)
            .label("")
            .ratio(1.0)
            .filled_symbol("日"),
        4,
        1,
    );
    assert_eq!(grid.row_display(0, &clusters), " ---");
    let (grid, clusters) = render(
        &LineGauge::new()
            .value_label("")
            .subcell(false)
            .ratio(f64::NAN),
        10,
        1,
    );
    assert_eq!(grid.row_display(0, &clusters), "──────────");
}

#[test]
fn focused_and_disabled_styles_follow_theme_tokens() -> Result<(), String> {
    for variant in [Variant::Dark, Variant::Light] {
        let theme = Theme::builtin(variant);
        for state in [
            WidgetState::Normal,
            WidgetState::Focused,
            WidgetState::Disabled,
        ] {
            let mut grid = Grid::new(5, 1);
            let mut clusters = ClusterTable::new();
            LineGauge::new()
                .value_label("")
                .subcell(false)
                .label("")
                .ratio(1.0)
                .state(state)
                .render(
                    Rect::new(0, 0, 5, 1),
                    &mut RenderCx::new(&mut grid, &mut clusters, &theme),
                );
            let token = match state {
                WidgetState::Normal | WidgetState::Focused => Token::Fail,
                _ => Token::TextMuted,
            };
            let cell = grid.get(1, 0).ok_or("missing filled cell")?;
            assert_eq!(cell.style().fg, theme.resolve(token));
            assert_eq!(cell.style().attrs.dim(), state == WidgetState::Disabled);
        }
    }
    let (grid, _) = render(
        &LineGauge::new()
            .value_label("")
            .subcell(false)
            .label("")
            .ratio(1.0)
            .filled_style(Style::new().fg(Color::Indexed(2)))
            .state(WidgetState::Disabled),
        5,
        1,
    );
    assert!(
        grid.get(1, 0)
            .ok_or("missing filled cell")?
            .style()
            .attrs
            .dim()
    );
    Ok(())
}

#[test]
fn matches_ratatui_visible_cells_for_ratios_labels_and_line_sets() -> Result<(), String> {
    for width in [0, 1, 4, 5, 6, 7, 20, 80, 200] {
        for ratio in [0.0, 0.01, 0.25, 0.5, 0.99, 1.0] {
            for label in ["", "50%", "日本", "e\u{301}"] {
                for (set, symbol) in [
                    (LineSet::Normal, "─"),
                    (LineSet::Thick, "━"),
                    (LineSet::Double, "═"),
                    (LineSet::Ascii, "-"),
                ] {
                    let ours = LineGauge::new()
                        .value_label("")
                        .subcell(false)
                        .ratio(ratio)
                        .label(label)
                        .line_set(set)
                        .filled_symbol(symbol)
                        .style(Style::new().fg(Color::Indexed(3)))
                        .label_style(Style::new().fg(Color::Indexed(3)))
                        .filled_style(Style::new().fg(Color::Indexed(2)))
                        .unfilled_style(Style::new().fg(Color::Indexed(1)));
                    let (grid, clusters) = render(&ours, width, 2);
                    let area = RatRect::new(0, 0, width, 2);
                    let mut expected = Buffer::empty(area);
                    RatGauge::default()
                        .ratio(ratio)
                        .label(label)
                        .style(RatStyle::default().fg(RatColor::Indexed(3)))
                        .filled_symbol(symbol)
                        .unfilled_symbol(symbol)
                        .filled_style(RatStyle::default().fg(RatColor::Indexed(2)))
                        .unfilled_style(RatStyle::default().fg(RatColor::Indexed(1)))
                        .render(area, &mut expected);
                    for y in 0..2 {
                        for x in 0..width {
                            let cell = grid.get(x, y).ok_or("missing cell")?;
                            let symbol = match cell.glyph().decode() {
                                DecodedGlyph::Inline(c) => c.to_string(),
                                DecodedGlyph::Cluster(id) => clusters
                                    .cluster_text(id)
                                    .ok_or("missing cluster")?
                                    .to_owned(),
                                DecodedGlyph::Continuation => " ".to_owned(),
                            };
                            let want = &expected[(x, y)];
                            assert_eq!(
                                symbol,
                                want.symbol(),
                                "width={width}, label={label:?}, {x},{y}"
                            );
                            if matches!(cell.glyph().decode(), DecodedGlyph::Continuation) {
                                continue;
                            }
                            let expected_fg = if let RatColor::Indexed(fg) = want.fg {
                                Color::Indexed(fg)
                            } else {
                                Color::Default
                            };
                            assert_eq!(
                                cell.style().fg,
                                expected_fg,
                                "width={width} label={label:?} ratio={ratio} x={x} y={y} glyph={:?}",
                                cell.glyph().decode()
                            );
                            assert_eq!(cell.style().bg, Color::Default);
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

#[test]
fn default_threshold_colors_match_literal_graphite_values() -> Result<(), String> {
    for (variant, colors) in [
        (
            Variant::Dark,
            [
                Color::Rgb(98, 168, 255),
                Color::Rgb(242, 184, 75),
                Color::Rgb(255, 122, 107),
            ],
        ),
        (
            Variant::Light,
            [
                Color::Rgb(79, 137, 209),
                Color::Rgb(184, 122, 10),
                Color::Rgb(194, 57, 43),
            ],
        ),
    ] {
        let theme = Theme::builtin(variant);
        for (ratio, color) in [0.799, 0.8, 0.95].into_iter().zip(colors) {
            let mut grid = Grid::new(8, 1);
            let mut clusters = ClusterTable::new();
            LineGauge::new()
                .value_label("")
                .subcell(false)
                .label("")
                .ratio(ratio)
                .render(
                    Rect::new(0, 0, 8, 1),
                    &mut RenderCx::new(&mut grid, &mut clusters, &theme),
                );
            assert_eq!(
                grid.get(1, 0).ok_or("missing filled cell")?.style().fg,
                color
            );
        }
    }
    Ok(())
}

#[test]
fn design_layout_keeps_value_right_and_uses_fractional_head() {
    for (ratio, label, width, expected) in [
        (0.5, None, 20, "━━━━━━━━──────── 50%"),
        (0.53125, None, 20, "━━━━━━━━▌─────── 53%"),
        (0.5, Some("CPU"), 20, "CPU ━━━━━━────── 50%"),
        (0.5, None, 7, "━▌─ 50%"),
        (0.5, None, 3, "50%"),
        (0.5, None, 2, "50"),
        (0.5, None, 1, "5"),
        (0.0, None, 12, "───────── 0%"),
        (1.0, None, 12, "━━━━━━━ 100%"),
    ] {
        let mut gauge = LineGauge::new().ratio(ratio);
        if let Some(label) = label {
            gauge = gauge.label(label);
        }
        let (grid, clusters) = render(&gauge, width, 1);
        assert_eq!(grid.row_display(0, &clusters), expected);
    }
}

#[test]
fn repaint_clears_old_label_and_does_not_touch_other_rows() {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(20, 2);
    let mut clusters = ClusterTable::new();
    grid.put_str(0, 0, Style::new(), "sentinel", &mut clusters);
    let area = Rect::new(0, 1, 20, 1);
    LineGauge::new()
        .label("long")
        .ratio(0.5)
        .render(area, &mut RenderCx::new(&mut grid, &mut clusters, &theme));
    LineGauge::new()
        .label("x")
        .ratio(0.0)
        .render(area, &mut RenderCx::new(&mut grid, &mut clusters, &theme));
    assert_eq!(grid.row_display(1, &clusters), "x ─────────────── 0%");
    assert!(grid.row_display(0, &clusters).starts_with("sentinel"));
}

#[test]
fn prepared_ansi16_uses_semantic_indices_and_noncolor_focus() -> Result<(), &'static str> {
    use lunatui_theme::Profile;
    let theme = Theme::builtin(Variant::Dark);
    let palette = theme.prepare(Profile::Ansi16);
    for (state, ratio, expected_fill) in [
        (WidgetState::Normal, 0.5, 4),
        (WidgetState::Normal, 0.8, 3),
        (WidgetState::Normal, 0.95, 1),
        (WidgetState::Focused, 0.5, 4),
        (WidgetState::Disabled, 0.5, 8),
    ] {
        let mut grid = Grid::new(20, 1);
        let mut clusters = ClusterTable::new();
        LineGauge::new()
            .label("CPU")
            .ratio(ratio)
            .state(state)
            .render(
                Rect::new(0, 0, 20, 1),
                &mut RenderCx::new(&mut grid, &mut clusters, &theme).palette(&palette),
            );
        let filled = grid.get(4, 0).ok_or("missing track")?;
        assert_eq!(filled.style().fg, Color::Indexed(expected_fill));
        assert_eq!(filled.style().attrs.dim(), state == WidgetState::Disabled);
        let value = grid.get(17, 0).ok_or("missing value")?;
        assert_eq!(value.style().fg, Color::Indexed(8));
        assert_eq!(value.style().attrs.bold(), state == WidgetState::Focused);
        assert_eq!(value.style().attrs.reverse(), state == WidgetState::Focused);
    }
    Ok(())
}

#[test]
fn ascii_fallback_and_nonfinite_progress_have_literal_output() {
    let (grid, clusters) = render(
        &LineGauge::new()
            .label("CPU")
            .ratio(0.53125)
            .line_set(LineSet::Ascii),
        20,
        1,
    );
    assert_eq!(grid.row_display(0, &clusters), "CPU ######------ 53%");
    for (ratio, expected) in [
        (f64::NAN, "───────── 0%"),
        (f64::NEG_INFINITY, "───────── 0%"),
        (f64::INFINITY, "━━━━━━━ 100%"),
    ] {
        let (grid, clusters) = render(&LineGauge::new().ratio(ratio), 12, 1);
        assert_eq!(grid.row_display(0, &clusters), expected);
    }
}
