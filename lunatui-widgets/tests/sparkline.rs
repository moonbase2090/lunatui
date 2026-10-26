// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::{Attrs, ClusterTable, Color, Grid, Rect, Style};
use lunatui_theme::{Theme, Token, Variant};
use lunatui_widgets::{
    BarSet, Baseline, Block, NINE_LEVELS, RenderCx, RenderDirection, Sparkline, SparklineBar,
    THREE_LEVELS, Widget, WidgetState,
};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect as RatRect;
use ratatui::style::{Color as RatColor, Modifier, Style as RatStyle};
use ratatui::widgets::{Sparkline as RatSparkline, SparklineBar as RatBar, Widget as RatWidget};

fn paint(
    spark: &Sparkline<'_, impl lunatui_widgets::SparklineValue>,
    width: u16,
    height: u16,
) -> (Grid, ClusterTable) {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(width, height);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    spark.render(Rect::new(0, 0, width, height), &mut cx);
    (grid, clusters)
}

fn rat_paint(spark: &RatSparkline<'_>, width: u16, height: u16) -> Buffer {
    let area = RatRect::new(0, 0, width, height);
    let mut buffer = Buffer::empty(area);
    RatWidget::render(spark, area, &mut buffer);
    buffer
}

fn rows(grid: &Grid, clusters: &ClusterTable) -> Vec<String> {
    (0..grid.height())
        .map(|row| grid.row_display(row, clusters))
        .collect()
}

fn rat_rows(buffer: &Buffer) -> Vec<String> {
    (0..buffer.area.height)
        .map(|y| {
            (0..buffer.area.width)
                .map(|x| buffer[(x, y)].symbol().to_owned())
                .collect()
        })
        .collect()
}

fn assert_same_symbols(ours: &[String], theirs: &[String]) {
    assert_eq!(ours, theirs);
}

fn rat_color(color: RatColor) -> Color {
    match color {
        RatColor::Reset => Color::Default,
        RatColor::Rgb(red, green, blue) => Color::Rgb(red, green, blue),
        RatColor::Indexed(index) => Color::Indexed(index),
        RatColor::Black => Color::Indexed(0),
        RatColor::Red => Color::Indexed(1),
        RatColor::Green => Color::Indexed(2),
        RatColor::Yellow => Color::Indexed(3),
        RatColor::Blue => Color::Indexed(4),
        RatColor::Magenta => Color::Indexed(5),
        RatColor::Cyan => Color::Indexed(6),
        RatColor::Gray => Color::Indexed(7),
        RatColor::DarkGray => Color::Indexed(8),
        RatColor::LightRed => Color::Indexed(9),
        RatColor::LightGreen => Color::Indexed(10),
        RatColor::LightYellow => Color::Indexed(11),
        RatColor::LightBlue => Color::Indexed(12),
        RatColor::LightMagenta => Color::Indexed(13),
        RatColor::LightCyan => Color::Indexed(14),
        RatColor::White => Color::Indexed(15),
    }
}

fn assert_same_cells(grid: &Grid, clusters: &ClusterTable, buffer: &Buffer) -> Result<(), String> {
    assert_same_symbols(&rows(grid, clusters), &rat_rows(buffer));
    for y in 0..buffer.area.height {
        for x in 0..buffer.area.width {
            let Some(cell) = grid.get(x, y) else {
                return Err(format!("missing cell at {x},{y}"));
            };
            let want = &buffer[(x, y)];
            let style = cell.style();
            assert_eq!(style.fg, rat_color(want.fg), "fg at {x},{y}");
            assert_eq!(style.bg, rat_color(want.bg), "bg at {x},{y}");
            assert_eq!(
                style.attrs.bold(),
                want.modifier.contains(Modifier::BOLD),
                "bold at {x},{y}"
            );
            assert_eq!(
                style.attrs.dim(),
                want.modifier.contains(Modifier::DIM),
                "dim at {x},{y}"
            );
        }
    }
    Ok(())
}

#[test]
fn nine_level_bars_match_ratatui() -> Result<(), String> {
    let data = [0, 1, 2, 3, 4, 5, 6, 7, 8];
    let (grid, clusters) = paint(&Sparkline::new(&data).style(Style::new()), 12, 1);
    let buffer = rat_paint(&RatSparkline::default().data(data), 12, 1);
    assert_same_cells(&grid, &clusters, &buffer)?;
    assert_eq!(rows(&grid, &clusters), [" ▁▂▃▄▅▆▇█   "]);
    Ok(())
}

#[test]
fn double_height_bars_match_ratatui() -> Result<(), String> {
    let data = [0, 1, 2, 3, 4, 5, 6, 7, 8];
    let (grid, clusters) = paint(&Sparkline::new(&data).style(Style::new()), 12, 2);
    let buffer = rat_paint(&RatSparkline::default().data(data), 12, 2);
    assert_same_cells(&grid, &clusters, &buffer)?;
    assert_eq!(rows(&grid, &clusters), ["     ▂▄▆█   ", " ▂▄▆█████   "]);
    Ok(())
}

#[test]
fn right_to_left_matches_ratatui() -> Result<(), String> {
    let data = [0, 1, 2, 3, 4, 5, 6, 7, 8];
    let (grid, clusters) = paint(
        &Sparkline::new(&data)
            .style(Style::new())
            .direction(RenderDirection::RightToLeft),
        12,
        1,
    );
    let buffer = rat_paint(
        &RatSparkline::default()
            .data(data)
            .direction(ratatui::widgets::RenderDirection::RightToLeft),
        12,
        1,
    );
    assert_same_cells(&grid, &clusters, &buffer)?;
    assert_eq!(rows(&grid, &clusters), ["   █▇▆▅▄▃▂▁ "]);
    Ok(())
}

#[test]
fn explicit_max_and_zero_max_match_ratatui() -> Result<(), String> {
    let data = [0, 10, 5];
    let (grid, clusters) = paint(&Sparkline::new(&data).style(Style::new()).max(10), 10, 2);
    let buffer = rat_paint(&RatSparkline::default().data(data).max(10), 10, 2);
    assert_same_cells(&grid, &clusters, &buffer)?;

    let zeros = [0, 1, 2];
    let (grid, clusters) = paint(&Sparkline::new(&zeros).style(Style::new()).max(0), 6, 1);
    let buffer = rat_paint(&RatSparkline::default().data(zeros).max(0), 6, 1);
    assert_same_cells(&grid, &clusters, &buffer)?;
    assert_eq!(rows(&grid, &clusters), ["      "]);
    Ok(())
}

#[test]
fn extreme_values_keep_ratatui_precision() -> Result<(), String> {
    let data = [u64::MAX];
    let (grid, clusters) = paint(
        &Sparkline::new(&data).style(Style::new()).max(u64::MAX),
        1,
        3,
    );
    let buffer = rat_paint(&RatSparkline::default().data(data).max(u64::MAX), 1, 3);
    assert_same_cells(&grid, &clusters, &buffer)?;
    assert_eq!(rows(&grid, &clusters), ["█", "█", "█"]);

    let data = [u64::MAX - 1];
    let (grid, clusters) = paint(
        &Sparkline::new(&data).style(Style::new()).max(u64::MAX),
        1,
        1,
    );
    let buffer = rat_paint(&RatSparkline::default().data(data).max(u64::MAX), 1, 1);
    assert_same_cells(&grid, &clusters, &buffer)?;
    assert_eq!(rows(&grid, &clusters), ["▇"]);
    Ok(())
}

#[test]
fn absent_samples_match_ratatui() -> Result<(), String> {
    let data = [
        None,
        Some(1),
        Some(2),
        Some(3),
        Some(4),
        Some(5),
        Some(6),
        Some(7),
        Some(8),
    ];
    let red = Style::new().fg(Color::Rgb(200, 40, 40));
    let rat_red = RatStyle::default().fg(RatColor::Rgb(200, 40, 40));
    let (grid, clusters) = paint(
        &Sparkline::new(&data)
            .style(Style::new())
            .absent_value_style(red)
            .absent_value_symbol("█"),
        12,
        2,
    );
    let buffer = rat_paint(
        &RatSparkline::default()
            .data(data)
            .absent_value_style(rat_red)
            .absent_value_symbol("█"),
        12,
        2,
    );
    assert_same_cells(&grid, &clusters, &buffer)?;
    assert_eq!(rows(&grid, &clusters), ["█    ▂▄▆█   ", "█▂▄▆█████   "]);
    Ok(())
}

#[test]
fn per_bar_styles_match_ratatui() -> Result<(), String> {
    let data = [
        SparklineBar::value(1).style(Style::new().fg(Color::Rgb(10, 20, 30))),
        SparklineBar::value(4).style(Style::new().fg(Color::Rgb(40, 50, 60))),
        SparklineBar::value(8).style(Style::new().bg(Color::Rgb(70, 80, 90))),
    ];
    let rat = [
        RatBar::from(1).style(RatStyle::default().fg(RatColor::Rgb(10, 20, 30))),
        RatBar::from(4).style(RatStyle::default().fg(RatColor::Rgb(40, 50, 60))),
        RatBar::from(8).style(RatStyle::default().bg(RatColor::Rgb(70, 80, 90))),
    ];
    let (grid, clusters) = paint(&Sparkline::new(&data).style(Style::new()), 3, 1);
    let buffer = rat_paint(&RatSparkline::default().data(rat), 3, 1);
    assert_same_cells(&grid, &clusters, &buffer)?;
    Ok(())
}

#[test]
fn three_levels_and_bordered_block_match_ratatui() -> Result<(), String> {
    let data = [0, 2, 4, 6, 8];
    let (grid, clusters) = paint(
        &Sparkline::new(&data)
            .style(Style::new())
            .bar_set(THREE_LEVELS),
        6,
        1,
    );
    let buffer = rat_paint(
        &RatSparkline::default()
            .data(data)
            .bar_set(ratatui::symbols::bar::THREE_LEVELS),
        6,
        1,
    );
    assert_same_cells(&grid, &clusters, &buffer)?;

    let (grid, clusters) = paint(
        &Sparkline::new(&data)
            .style(Style::new().fg(Color::Rgb(1, 2, 3)))
            .block(Block::bordered().title("load")),
        12,
        4,
    );
    let buffer = rat_paint(
        &RatSparkline::default()
            .data(data)
            .style(RatStyle::default().fg(RatColor::Rgb(1, 2, 3)))
            .block(ratatui::widgets::Block::bordered().title("load")),
        12,
        4,
    );
    assert_same_symbols(&rows(&grid, &clusters), &rat_rows(&buffer));
    Ok(())
}

#[test]
fn default_bar_set_is_nine_levels() {
    assert_eq!(BarSet::default(), NINE_LEVELS);
}

#[test]
fn minimum_baseline_lifts_the_floor() {
    let data = [2, 4, 6];
    let (zero, clusters) = paint(&Sparkline::new(&data).style(Style::new()), 3, 1);
    let (lifted, _) = paint(
        &Sparkline::new(&data)
            .style(Style::new())
            .baseline(Baseline::Minimum),
        3,
        1,
    );
    assert_eq!(rows(&zero, &clusters), ["▂▅█"]);
    assert_eq!(rows(&lifted, &clusters), [" ▄█"]);
}

#[test]
fn themed_states_use_distinct_tokens() -> Result<(), String> {
    let data = [1, 2, 8];
    let theme = Theme::builtin(Variant::Dark);
    for state in [
        WidgetState::Normal,
        WidgetState::Focused,
        WidgetState::Disabled,
    ] {
        let (grid, _) = paint(&Sparkline::new(&data).state(state), 3, 1);
        let Some(last) = grid.get(2, 0).map(lunatui_core::Cell::style) else {
            return Err("missing last cell".to_owned());
        };
        let Some(first) = grid.get(0, 0).map(lunatui_core::Cell::style) else {
            return Err("missing first cell".to_owned());
        };
        match state {
            WidgetState::Normal => {
                assert_eq!(first.fg, theme.resolve(Token::TextPrimary));
                assert_eq!(last.fg, theme.resolve(Token::Accent));
                assert!(!last.attrs.bold());
            }
            WidgetState::Focused => {
                assert_eq!(first.fg, theme.resolve(Token::Accent));
                assert_eq!(last.fg, theme.resolve(Token::FocusRing));
                assert!(last.attrs.bold());
            }
            WidgetState::Disabled => {
                assert_eq!(first.fg, theme.resolve(Token::TextPrimary));
                assert_eq!(last.fg, theme.resolve(Token::TextPrimary));
                assert!(last.attrs.dim());
                assert_eq!(last.attrs, Attrs::new().set_dim(true));
            }
            _ => return Err("untested state".to_owned()),
        }
    }
    Ok(())
}

#[test]
fn measure_follows_the_sample_count() {
    let data = [1, 2, 3, 4];
    assert_eq!(
        Sparkline::new(&data).measure(lunatui_widgets::Size::new(80, 24)),
        lunatui_widgets::Size::new(4, 1)
    );
    assert_eq!(
        Sparkline::new(&data).measure(lunatui_widgets::Size::new(2, 0)),
        lunatui_widgets::Size::new(0, 0)
    );
}

#[test]
fn zero_area_leaves_the_grid_blank() {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(4, 2);
    let mut clusters = ClusterTable::new();
    let spark = Sparkline::new(&[1, 2, 3]).state(WidgetState::Focused);
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    spark.render(Rect::new(1, 1, 0, 1), &mut cx);
    assert_eq!(grid.row_display(1, &clusters), "    ");
}
