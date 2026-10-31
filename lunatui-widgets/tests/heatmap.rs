// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::{Cell, ClusterTable, Color, Grid, Rect, Size, Style};
use lunatui_theme::{Theme, Token, Variant};
use lunatui_widgets::{Heatmap, HeatmapCell, HeatmapRow, RenderCx, Widget, WidgetState};

fn draw(map: &Heatmap<'_>, area: Rect, width: u16, height: u16) -> (Grid, ClusterTable) {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(width, height);
    let mut clusters = ClusterTable::new();
    grid.fill(
        Rect::new(0, 0, width, height),
        Style::new(),
        '#',
        &mut clusters,
    );
    map.render(area, &mut RenderCx::new(&mut grid, &mut clusters, &theme));
    (grid, clusters)
}

fn background(grid: &Grid, x: u16, y: u16) -> Option<Color> {
    grid.get(x, y).map(|cell| cell.style().bg)
}

fn color(token: Token) -> Color {
    Theme::builtin(Variant::Dark).resolve(token)
}

#[test]
fn fixed_range_draws_each_semantic_band() {
    let values = [Some(0), Some(25), Some(50), Some(75), Some(100)];
    let rows = [HeatmapRow::new("", &values)];
    let map = Heatmap::new(&rows).range(0..=100);
    let (grid, clusters) = draw(&map, Rect::new(0, 0, 10, 1), 10, 1);
    assert_eq!(grid.row_display(0, &clusters), "          ");
    for (x, token) in [
        (0, Token::Surface1),
        (2, Token::Accent),
        (4, Token::Ok),
        (6, Token::Warn),
        (8, Token::Fail),
    ] {
        assert_eq!(background(&grid, x, 0), Some(color(token)));
        assert_eq!(background(&grid, x + 1, 0), Some(color(token)));
    }
}

#[test]
fn missing_cells_and_ragged_tails_are_distinct_from_zero() {
    let first = [Some(0), None, Some(100)];
    let second = [Some(50)];
    let rows = [HeatmapRow::new("", &first), HeatmapRow::new("", &second)];
    let map = Heatmap::new(&rows).range(0..=100);
    assert_eq!(map.shape(), (2, 3));
    let (grid, clusters) = draw(&map, Rect::new(0, 0, 6, 2), 6, 2);
    assert_eq!(grid.row_display(0, &clusters), "  ·   ");
    assert_eq!(grid.row_display(1, &clusters), "  · · ");
    assert_eq!(background(&grid, 0, 0), Some(color(Token::Surface1)));
    assert_eq!(background(&grid, 2, 0), Some(color(Token::Surface0)));
    assert_eq!(background(&grid, 0, 1), Some(color(Token::Ok)));
    assert_eq!(
        map.cell_at(Rect::new(0, 0, 6, 2), 5, 1),
        Some(HeatmapCell { row: 1, column: 2 })
    );
}

#[test]
fn automatic_scale_ignores_missing_values() {
    let values = [None, Some(10), Some(20), Some(30)];
    let rows = [HeatmapRow::new("", &values)];
    let (grid, clusters) = draw(&Heatmap::new(&rows), Rect::new(0, 0, 8, 1), 8, 1);
    assert_eq!(grid.row_display(0, &clusters), "·       ");
    assert_eq!(background(&grid, 2, 0), Some(color(Token::Surface1)));
    assert_eq!(background(&grid, 4, 0), Some(color(Token::Ok)));
    assert_eq!(background(&grid, 6, 0), Some(color(Token::Fail)));
}

#[test]
fn reversed_bounds_clamp_both_extremes() {
    let values = [Some(0), Some(50), Some(u64::MAX)];
    let rows = [HeatmapRow::new("", &values)];
    let (grid, _) = draw(
        &Heatmap::new(&rows).range(std::ops::RangeInclusive::new(75, 25)),
        Rect::new(0, 0, 6, 1),
        6,
        1,
    );
    assert_eq!(background(&grid, 0, 0), Some(color(Token::Surface1)));
    assert_eq!(background(&grid, 2, 0), Some(color(Token::Ok)));
    assert_eq!(background(&grid, 4, 0), Some(color(Token::Fail)));
}

#[test]
fn scale_arithmetic_handles_the_full_u64_domain() {
    let values = [Some(0), Some(u64::MAX / 2 + 1), Some(u64::MAX)];
    let rows = [HeatmapRow::new("", &values)];
    let (grid, _) = draw(&Heatmap::new(&rows), Rect::new(0, 0, 6, 1), 6, 1);
    assert_eq!(background(&grid, 0, 0), Some(color(Token::Surface1)));
    assert_eq!(background(&grid, 2, 0), Some(color(Token::Ok)));
    assert_eq!(background(&grid, 4, 0), Some(color(Token::Fail)));
}

#[test]
fn every_twenty_percent_boundary_changes_the_band() {
    let values = [
        Some(19),
        Some(20),
        Some(39),
        Some(40),
        Some(59),
        Some(60),
        Some(79),
        Some(80),
    ];
    let rows = [HeatmapRow::new("", &values)];
    let (grid, _) = draw(
        &Heatmap::new(&rows).range(0..=100),
        Rect::new(0, 0, 16, 1),
        16,
        1,
    );
    for (x, token) in [
        (0, Token::Surface1),
        (2, Token::Accent),
        (4, Token::Accent),
        (6, Token::Ok),
        (8, Token::Ok),
        (10, Token::Warn),
        (12, Token::Warn),
        (14, Token::Fail),
    ] {
        assert_eq!(background(&grid, x, 0), Some(color(token)));
    }
}

#[test]
fn constant_and_all_missing_matrices_have_defined_output() {
    let constant = [Some(7), Some(7)];
    let missing = [None, None];
    let rows = [
        HeatmapRow::new("", &constant),
        HeatmapRow::new("", &missing),
    ];
    let (grid, clusters) = draw(&Heatmap::new(&rows), Rect::new(0, 0, 4, 2), 4, 2);
    assert_eq!(grid.row_display(0, &clusters), "    ");
    assert_eq!(grid.row_display(1, &clusters), "· · ");
    assert_eq!(background(&grid, 0, 0), Some(color(Token::Surface1)));
    assert_eq!(background(&grid, 0, 1), Some(color(Token::Surface0)));
}

#[test]
fn offset_labels_partial_cells_and_hit_testing_share_geometry() {
    let values = [Some(0), Some(100)];
    let rows = [
        HeatmapRow::new("api", &values),
        HeatmapRow::new("job", &values),
    ];
    let map = Heatmap::new(&rows)
        .column_labels(&["00", "01"])
        .cell_size(Size::new(3, 2));
    let area = Rect::new(10, 20, 9, 5);
    assert_eq!(map.measure(Size::new(100, 100)), Size::new(10, 5));
    assert_eq!(map.measure(Size::new(9, 4)), Size::new(9, 4));
    assert_eq!(
        map.cell_area(area, HeatmapCell { row: 0, column: 0 }),
        Some(Rect::new(14, 21, 3, 2))
    );
    assert_eq!(
        map.cell_area(area, HeatmapCell { row: 1, column: 1 }),
        Some(Rect::new(17, 23, 2, 2))
    );
    assert_eq!(
        map.cell_at(area, 14, 21),
        Some(HeatmapCell { row: 0, column: 0 })
    );
    assert_eq!(
        map.cell_at(area, 18, 24),
        Some(HeatmapCell { row: 1, column: 1 })
    );
    for (x, y) in [(13, 21), (14, 20), (19, 24), (18, 25)] {
        assert_eq!(map.cell_at(area, x, y), None);
    }
    let (grid, clusters) = draw(&map, area, 20, 26);
    assert_eq!(grid.row_display(20, &clusters), "##########    00 01#");
    assert_eq!(grid.row_display(21, &clusters), "##########api      #");
    assert_eq!(grid.row_display(23, &clusters), "##########job      #");
    assert_eq!(grid.row_display(25, &clusters), "####################");
}

#[test]
fn wide_labels_are_measured_in_terminal_columns() {
    let values = [Some(10)];
    let rows = [HeatmapRow::new("界界", &values)];
    let map = Heatmap::new(&rows).column_labels(&["界界"]);
    assert_eq!(map.measure(Size::new(20, 10)), Size::new(7, 2));
    let (grid, clusters) = draw(&map, Rect::new(0, 0, 7, 2), 8, 2);
    assert_eq!(grid.row_display(0, &clusters), "     界#");
    assert_eq!(grid.row_display(1, &clusters), "界界   #");
    assert_eq!(map.cell_at(Rect::new(0, 0, 7, 2), 4, 1), None);
    assert_eq!(
        map.cell_at(Rect::new(0, 0, 7, 2), 5, 1),
        Some(HeatmapCell { row: 0, column: 0 })
    );
}

#[test]
fn selection_keeps_the_value_background_and_uses_focus_tokens() -> Result<(), &'static str> {
    let values = [Some(100)];
    let rows = [HeatmapRow::new("", &values)];
    let map = Heatmap::new(&rows)
        .range(0..=100)
        .selected(Some(HeatmapCell { row: 0, column: 0 }));
    for state in [WidgetState::Normal, WidgetState::Focused] {
        let (grid, clusters) = draw(&map.clone().state(state), Rect::new(0, 0, 2, 1), 2, 1);
        assert_eq!(grid.row_display(0, &clusters), "◆ ");
        let style = grid
            .get(0, 0)
            .map(Cell::style)
            .ok_or("missing selected cell")?;
        assert_eq!(style.bg, color(Token::Fail));
        assert_eq!(
            style.fg,
            color(if state == WidgetState::Focused {
                Token::FocusRing
            } else {
                Token::TextPrimary
            })
        );
        assert_eq!(style.attrs.bold(), state == WidgetState::Focused);
    }
    Ok(())
}

#[test]
fn disabled_maps_use_foreground_swatches_and_suppress_selection() -> Result<(), &'static str> {
    let values = [Some(100), None];
    let rows = [HeatmapRow::new("", &values)];
    let map = Heatmap::new(&rows)
        .range(0..=100)
        .selected(Some(HeatmapCell { row: 0, column: 0 }))
        .state(WidgetState::Disabled);
    let (grid, clusters) = draw(&map, Rect::new(0, 0, 4, 1), 4, 1);
    assert_eq!(grid.row_display(0, &clusters), "░░· ");
    let style = grid
        .get(0, 0)
        .map(Cell::style)
        .ok_or("missing disabled cell")?;
    assert_eq!(style.bg, color(Token::Surface0));
    assert_eq!(style.fg, color(Token::Fail));
    assert!(style.attrs.dim());
    Ok(())
}

#[test]
fn empty_zero_sized_and_hidden_cells_do_not_touch_neighbors() {
    let values = [Some(0)];
    let rows = [HeatmapRow::new("", &values)];
    let map = Heatmap::new(&rows).cell_size(Size::new(0, 0));
    assert_eq!(map.measure(Size::new(9, 9)), Size::new(1, 1));
    assert_eq!(Heatmap::new(&[]).shape(), (0, 0));
    assert_eq!(Heatmap::new(&[]).measure(Size::new(9, 9)), Size::new(0, 0));
    for area in [Rect::new(0, 0, 0, 1), Rect::new(0, 0, 1, 0)] {
        let (grid, clusters) = draw(&map, area, 3, 1);
        assert_eq!(grid.row_display(0, &clusters), "###");
        assert_eq!(map.cell_area(area, HeatmapCell { row: 0, column: 0 }), None);
    }
    let area = Rect::new(u16::MAX - 1, u16::MAX - 1, 10, 10);
    assert_eq!(
        map.cell_area(area, HeatmapCell { row: 0, column: 0 }),
        Some(Rect::new(u16::MAX - 1, u16::MAX - 1, 1, 1))
    );
    assert_eq!(
        map.cell_area(
            area,
            HeatmapCell {
                row: usize::MAX,
                column: usize::MAX
            }
        ),
        None
    );
}

#[cfg(feature = "color-profiles")]
#[test]
fn light_and_dark_prepared_ansi16_palettes_reach_the_cells() -> Result<(), &'static str> {
    use lunatui_theme::Profile;
    let values = [Some(100)];
    let rows = [HeatmapRow::new("", &values)];
    for variant in [Variant::Dark, Variant::Light] {
        let mut theme = Theme::builtin(variant);
        theme.set_token(Token::Fail, Color::Rgb(205, 0, 0));
        let palette = theme.prepare(Profile::Ansi16);
        let mut grid = Grid::new(2, 1);
        let mut clusters = ClusterTable::new();
        Heatmap::new(&rows).range(0..=100).render(
            Rect::new(0, 0, 2, 1),
            &mut RenderCx::new(&mut grid, &mut clusters, &theme).palette(&palette),
        );
        assert_eq!(
            grid.get(0, 0).ok_or("missing heat cell")?.style().bg,
            Color::Indexed(1)
        );
    }
    Ok(())
}
