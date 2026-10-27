// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::{Attrs, Cell, ClusterTable, Color, Grid, Rect, Style};
use lunatui_theme::{Theme, Variant};
use lunatui_widgets::{
    Gauge, RenderCx, Spinner, SpinnerPattern, StatusBar, Tabs, TextInput, Widget,
};

#[test]
fn tabs_and_status_render() {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(40, 3);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    Tabs::new(["One", "Two"])
        .select(1)
        .render(Rect::new(0, 0, 20, 1), &mut cx);
    StatusBar::new("ready").render(Rect::new(0, 2, 40, 1), &mut cx);
    assert!(grid.row_display(0, &clusters).contains("Two"));
    assert!(grid.row_display(2, &clusters).contains("ready"));
}

#[test]
fn gauge_and_spinner_render() {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(12, 2);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    Gauge::new(0.5).render(Rect::new(0, 0, 10, 1), &mut cx);
    Spinner::new(3).render(Rect::new(10, 0, 1, 1), &mut cx);
    assert!(grid.row_display(0, &clusters).contains('█'));
}

#[test]
fn spinner_patterns_render_the_target_frames() {
    let cases = [
        (SpinnerPattern::BrailleDots, "⠋", 1),
        (SpinnerPattern::HalfMoon, "◐", 1),
        (SpinnerPattern::Quadrant, "▖", 1),
        (SpinnerPattern::BrailleRing, "⣾", 1),
        (SpinnerPattern::Arc, "◜", 1),
        (SpinnerPattern::Bars, "▁▃▅", 3),
    ];
    for (pattern, expected, columns) in cases {
        let theme = Theme::builtin(Variant::Dark);
        let mut grid = Grid::new(3, 1);
        let mut clusters = ClusterTable::new();
        let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
        let spinner = Spinner::new(0).pattern(pattern);
        assert_eq!(
            spinner.measure(lunatui_core::Size::new(3, 1)).width,
            columns
        );
        spinner.render(Rect::new(0, 0, columns, 1), &mut cx);
        assert!(grid.row_display(0, &clusters).starts_with(expected));
    }
}

fn spinner_row(pattern: SpinnerPattern, frame: u32) -> String {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(3, 1);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    Spinner::new(frame)
        .pattern(pattern)
        .render(Rect::new(0, 0, 3, 1), &mut cx);
    grid.row_display(0, &clusters)
}

#[test]
fn spinner_cycles_wrap_at_each_patterns_frame_count() {
    assert_eq!(spinner_row(SpinnerPattern::BrailleRing, 0), "⣾  ");
    assert_eq!(spinner_row(SpinnerPattern::BrailleRing, 7), "⣷  ");
    assert_eq!(spinner_row(SpinnerPattern::BrailleRing, 8), "⣾  ");

    assert_eq!(spinner_row(SpinnerPattern::Bars, 0), "▁▃▅");
    assert_eq!(spinner_row(SpinnerPattern::Bars, 5), "▃▁▃");
    assert_eq!(spinner_row(SpinnerPattern::Bars, 6), "▁▃▅");
}

#[test]
fn text_input_caret_aligns_after_wide_grapheme() {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(4, 1);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    let value = "界abcd";
    let cursor = "界ab".len();
    TextInput::new(value, cursor).render(Rect::new(0, 0, 4, 1), &mut cx);
    let row = grid.row_display(0, &clusters);
    assert_eq!(row.find('│'), Some(2));
}

#[test]
fn text_input_renders_cursor() {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(10, 1);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    TextInput::new("hi", 2).render(Rect::new(0, 0, 10, 1), &mut cx);
    assert!(grid.row_display(0, &clusters).contains('│'));
}

#[test]
fn text_input_supports_a_styled_block_cursor() {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(12, 1);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    let background = Color::Rgb(185, 167, 240);
    TextInput::new("moon-dash", 9)
        .cursor_glyph(" ")
        .cursor_style(Style::new().fg(Color::Rgb(10, 13, 28)).bg(background))
        .render(Rect::new(0, 0, 12, 1), &mut cx);
    assert_eq!(grid.row_display(0, &clusters), "moon-dash   ");
    assert_eq!(grid.get(9, 0).map(|cell| cell.style().bg), Some(background));
}

#[test]
fn text_input_clips_a_wide_cursor_at_the_inner_edge() {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(3, 1);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    TextInput::new("a", 1)
        .cursor_glyph("界")
        .render(Rect::new(0, 0, 2, 1), &mut cx);
    assert_eq!(grid.row_display(0, &clusters), "a  ");
}

#[test]
fn lightcycle_raster_advances_with_virtual_time_and_wraps() {
    use lunatui_test::Clock;
    use lunatui_widgets::Lightcycle;
    use std::time::Duration;

    fn render(elapsed: Duration) -> (String, lunatui_core::Grid) {
        let theme = Theme::builtin(Variant::Dark);
        let mut grid = lunatui_core::Grid::new(8, 4);
        let mut clusters = lunatui_core::ClusterTable::new();
        let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
        Lightcycle::new(elapsed)
            .period(Duration::from_secs(4))
            .render(Rect::new(0, 0, 8, 4), &mut cx);
        (grid.row_display(0, &clusters), grid)
    }

    let start = Clock::new();
    let (_, start_grid) = render(start.now());
    let mut midpoint = start;
    midpoint.advance(Duration::from_millis(500));
    let (_, midpoint_grid) = render(midpoint.now());
    assert_ne!(start_grid, midpoint_grid);

    let mut wrapped = Clock::new();
    wrapped.advance(Duration::from_secs(4));
    let (_, wrapped_grid) = render(wrapped.now());
    assert_eq!(start_grid, wrapped_grid);
}

#[test]
fn lightcycle_raster_changes_at_sixty_hertz_virtual_time() {
    use lunatui_test::Clock;
    use std::time::Duration;

    fn render(elapsed: Duration) -> lunatui_core::Grid {
        let theme = Theme::builtin(Variant::Dark);
        let mut grid = lunatui_core::Grid::new(8, 4);
        let mut clusters = lunatui_core::ClusterTable::new();
        let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
        lunatui_widgets::Lightcycle::new(elapsed).render(Rect::new(0, 0, 8, 4), &mut cx);
        grid
    }

    let start = render(Clock::new().now());
    let next_frame = render(Clock::at(Duration::from_nanos(16_666_667)).now());
    assert_ne!(start, next_frame);
}

#[test]
fn lightcycle_raster_colors_the_trail_and_handles_empty_areas() {
    use lunatui_core::{Color, Style};
    use lunatui_widgets::Lightcycle;
    use std::time::Duration;

    let theme = Theme::builtin(Variant::Dark);
    let mut grid = lunatui_core::Grid::new(8, 4);
    let mut clusters = lunatui_core::ClusterTable::new();
    {
        let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
        Lightcycle::new(Duration::from_millis(250))
            .trail(
                Style::new().fg(Color::Rgb(255, 0, 0)),
                Style::new().fg(Color::Rgb(0, 0, 255)),
            )
            .render(Rect::new(0, 0, 8, 4), &mut cx);
    }
    assert!(
        grid.row_display(0, &clusters)
            .chars()
            .any(|ch| ('\u{2800}'..='\u{28ff}').contains(&ch))
    );
    let mut colors = std::collections::HashSet::new();
    for y in 0..grid.height() {
        for x in 0..grid.width() {
            if let Some(cell) = grid.get(x, y)
                && (0x2800..=0x28ff).contains(&cell.glyph().bits())
            {
                colors.insert(cell.style().fg);
            }
        }
    }
    assert!(
        colors.len() > 1,
        "trail should contain interpolated cell colors"
    );

    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    Lightcycle::new(Duration::ZERO).render(Rect::new(0, 0, 0, 0), &mut cx);
}

#[test]
fn put_str_keeps_text_attributes_across_wide_glyphs() {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(8, 1);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    let bold = Style::new()
        .fg(Color::Rgb(185, 167, 240))
        .with_attrs(Attrs::new().set_bold(true));
    cx.put_str(1, 0, "a界b", bold);
    let bold_at = |x| grid.get(x, 0).map(|cell| cell.style().attrs.bold());
    assert_eq!(bold_at(0), Some(false));
    assert_eq!(bold_at(1), Some(true));
    assert_eq!(bold_at(2), Some(true));
    assert_eq!(bold_at(4), Some(true));
    assert_eq!(bold_at(5), Some(false));
    assert_eq!(grid.row_display(0, &clusters).trim_end(), " a界b");
}

#[test]
fn tabs_highlight_style_bolds_only_the_selected_title() {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(12, 1);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    Tabs::new(["One", "Two"])
        .select(1)
        .highlight_style(Style::new().with_attrs(Attrs::new().set_bold(true)))
        .render(Rect::new(0, 0, 12, 1), &mut cx);
    let bold_at = |x| grid.get(x, 0).map(|cell| cell.style().attrs.bold());
    assert_eq!(bold_at(0), Some(false));
    assert_eq!(bold_at(4), Some(true));
    assert_eq!(bold_at(6), Some(true));
}

#[test]
fn status_bar_segments_fill_the_row_and_pin_right_segments_to_the_edge() {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(26, 1);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    cx.put_str(0, 0, "xxxxxxxxxxxxxxxxxxxxxxxxxx", Style::new());
    let panel = Color::Rgb(17, 22, 49);
    let mode = Style::new()
        .fg(Color::Rgb(10, 13, 28))
        .bg(Color::Rgb(185, 167, 240))
        .with_attrs(Attrs::new().set_bold(true));
    let percent = Style::new()
        .fg(Color::Rgb(10, 13, 28))
        .bg(Color::Rgb(243, 226, 176));
    StatusBar::new("")
        .style(Style::new().bg(panel))
        .left([(" NORMAL ", mode), (" main ", Style::new().bg(panel))])
        .right([(" rs ", Style::new().bg(panel)), (" 50% ", percent)])
        .render(Rect::new(1, 0, 24, 1), &mut cx);
    assert_eq!(grid.row_display(0, &clusters), "x NORMAL  main   rs  50% x");
    let style = |x| grid.get(x, 0).map(Cell::style).unwrap_or_default();
    assert!(style(2).attrs.bold());
    assert_eq!(style(2).bg, Color::Rgb(185, 167, 240));
    assert_eq!(
        style(16).bg,
        panel,
        "the gap between groups takes the bar style"
    );
    assert_eq!(
        style(22).bg,
        Color::Rgb(243, 226, 176),
        "right group ends at the edge"
    );
    assert_eq!(grid.row_display(0, &clusters).chars().last(), Some('x'));
}

#[test]
fn status_bar_text_clears_the_rest_of_its_row() {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(12, 1);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    cx.put_str(0, 0, "xxxxxxxxxxxx", Style::new());
    let panel = Color::Rgb(17, 22, 49);
    StatusBar::new("")
        .left([(" NORMAL ", Style::new().bg(Color::Rgb(185, 167, 240)))])
        .render(Rect::new(1, 0, 10, 1), &mut cx);
    StatusBar::new("ok")
        .style(Style::new().bg(panel))
        .render(Rect::new(1, 0, 10, 1), &mut cx);
    assert_eq!(grid.row_display(0, &clusters), "xok        x");
    let style = |x| grid.get(x, 0).map(Cell::style).unwrap_or_default();
    for x in 1..11 {
        assert_eq!(style(x).bg, panel, "x={x}");
    }
}
