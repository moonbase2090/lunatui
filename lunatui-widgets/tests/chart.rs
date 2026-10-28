// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::{Cell, ClusterTable, Color, Grid, Rect, Style};
use lunatui_layout::Constraint;
use lunatui_theme::{Theme, Token, Variant};
use lunatui_widgets::{
    Axis, Chart, Dataset, GraphType, LegendPosition, Marker, RenderCx, Widget, WidgetState,
};

fn paint(chart: &Chart<'_>, width: u16, height: u16) -> (Grid, ClusterTable) {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(width, height);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    chart.render(Rect::new(0, 0, width, height), &mut cx);
    (grid, clusters)
}

fn rows(chart: &Chart<'_>, width: u16, height: u16) -> Vec<String> {
    let (grid, clusters) = paint(chart, width, height);
    (0..height)
        .map(|row| grid.row_display(row, &clusters))
        .collect()
}

fn style_at(grid: &Grid, x: u16, y: u16) -> Style {
    grid.get(x, y).map(Cell::style).unwrap_or_default()
}

fn token(token: Token) -> Color {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(1, 1);
    let mut clusters = ClusterTable::new();
    RenderCx::new(&mut grid, &mut clusters, &theme).token(token)
}

fn unit_axes(chart: Chart<'_>) -> Chart<'_> {
    chart
        .x_axis(Axis::new().bounds([0.0, 1.0]))
        .y_axis(Axis::new().bounds([0.0, 1.0]))
}

const DIAGONAL: [(f64, f64); 3] = [(0.0, 0.0), (1.0, 1.0), (0.5, 0.5)];

#[test]
fn an_empty_chart_is_the_axes() {
    let chart = unit_axes(Chart::new(&[]));
    assert_eq!(rows(&chart, 6, 3), ["│     ", "│     ", "└─────"]);
}

#[test]
fn block_scatter_points_land_on_cells() {
    let sets = [Dataset::new(&DIAGONAL).marker(Marker::Block)];
    assert_eq!(
        rows(&unit_axes(Chart::new(&sets)), 6, 4),
        ["│    █", "│  █  ", "│█    ", "└─────"]
    );
}

#[test]
fn dot_markers_draw_a_bullet() {
    let sets = [Dataset::new(&DIAGONAL).marker(Marker::Dot)];
    assert_eq!(
        rows(&unit_axes(Chart::new(&sets)), 6, 4),
        ["│    •", "│  •  ", "│•    ", "└─────"]
    );
}

#[test]
fn half_blocks_split_a_cell_in_two() {
    let low_high = [(0.0, 0.0), (1.0, 1.0)];
    let sets = [Dataset::new(&low_high).marker(Marker::HalfBlock)];
    assert_eq!(
        rows(&unit_axes(Chart::new(&sets)), 4, 3),
        ["│  ▀", "│▄  ", "└───"]
    );
    let both = [(0.0, 0.0), (0.0, 1.0)];
    let sets = [Dataset::new(&both).marker(Marker::HalfBlock)];
    assert_eq!(rows(&unit_axes(Chart::new(&sets)), 4, 2), ["│█  ", "└───"]);
}

#[test]
fn braille_marks_one_dot_per_point() {
    let third = 1.0 / 3.0;
    let dots: Vec<(f64, f64)> = (0..4)
        .flat_map(|row| {
            [
                (0.0, 1.0 - f64::from(row) * third),
                (third, 1.0 - f64::from(row) * third),
            ]
        })
        .collect();
    let sets = [Dataset::new(&dots)];
    assert_eq!(rows(&unit_axes(Chart::new(&sets)), 3, 2), ["│⣿ ", "└──"]);
    let corner = [(2.0 * third, 1.0)];
    let sets = [Dataset::new(&corner)];
    assert_eq!(rows(&unit_axes(Chart::new(&sets)), 3, 2), ["│ ⠁", "└──"]);
    let single = [(0.0, 0.0)];
    let sets = [Dataset::new(&single)];
    assert_eq!(rows(&unit_axes(Chart::new(&sets)), 3, 2), ["│⡀ ", "└──"]);
}

#[test]
fn lines_join_points_cell_by_cell() {
    let sets = [Dataset::new(&DIAGONAL[..2])
        .marker(Marker::Block)
        .graph_type(GraphType::Line)];
    assert_eq!(
        rows(&unit_axes(Chart::new(&sets)), 6, 4),
        ["│    █", "│  ██ ", "│██   ", "└─────"]
    );
}

#[test]
fn a_single_point_line_is_a_point() {
    let one = [(0.5, 0.5)];
    let sets = [Dataset::new(&one)
        .marker(Marker::Block)
        .graph_type(GraphType::Line)];
    assert_eq!(rows(&unit_axes(Chart::new(&sets)), 4, 2), ["│ █ ", "└───"]);
}

#[test]
fn bars_rise_from_the_axis_in_eighths() {
    let data = [(0.0, 1.0), (0.5, 0.5), (1.0, 0.25)];
    let sets = [Dataset::new(&data).graph_type(GraphType::Bar)];
    assert_eq!(
        rows(&unit_axes(Chart::new(&sets)), 4, 3),
        ["│█  ", "│██▄", "└───"]
    );
}

#[test]
fn points_outside_the_bounds_are_dropped() {
    let data = [
        (-0.1, 0.5),
        (1.1, 0.5),
        (0.5, -0.1),
        (0.5, 1.1),
        (f64::NAN, 0.5),
        (0.5, f64::INFINITY),
    ];
    let sets = [Dataset::new(&data).marker(Marker::Block)];
    assert_eq!(
        rows(&unit_axes(Chart::new(&sets)), 4, 3),
        ["│   ", "│   ", "└───"]
    );
}

#[test]
fn lines_are_clipped_to_the_plot() {
    let across = [(-1.0, 0.5), (2.0, 0.5)];
    let sets = [Dataset::new(&across)
        .marker(Marker::Block)
        .graph_type(GraphType::Line)];
    assert_eq!(
        rows(&unit_axes(Chart::new(&sets)), 5, 2),
        ["│████", "└────"]
    );
    let outside = [(2.0, 0.0), (3.0, 1.0)];
    let sets = [Dataset::new(&outside)
        .marker(Marker::Block)
        .graph_type(GraphType::Line)];
    assert_eq!(
        rows(&unit_axes(Chart::new(&sets)), 5, 2),
        ["│    ", "└────"]
    );
}

#[test]
fn empty_bounds_plot_nothing() {
    let sets = [Dataset::new(&DIAGONAL).marker(Marker::Block)];
    for bounds in [[1.0, 1.0], [f64::NAN, 1.0]] {
        let chart = Chart::new(&sets)
            .x_axis(Axis::new().bounds(bounds))
            .y_axis(Axis::new().bounds([0.0, 1.0]));
        assert_eq!(rows(&chart, 4, 3), ["│   ", "│   ", "└───"]);
        let chart = Chart::new(&sets)
            .x_axis(Axis::new().bounds([0.0, 1.0]))
            .y_axis(Axis::new().bounds(bounds));
        assert_eq!(rows(&chart, 4, 3), ["│   ", "│   ", "└───"]);
    }
}

#[test]
fn reversed_bounds_flip_the_axis() {
    let sets = [Dataset::new(&DIAGONAL).marker(Marker::Block)];
    let chart = Chart::new(&sets)
        .x_axis(Axis::new().bounds([1.0, 0.0]))
        .y_axis(Axis::new().bounds([0.0, 1.0]));
    assert_eq!(rows(&chart, 4, 3), ["│█  ", "│ ██", "└───"]);
}

#[test]
fn bounds_rescale_the_data() {
    let data = [(10.0, 100.0), (20.0, 200.0)];
    let sets = [Dataset::new(&data).marker(Marker::Block)];
    let chart = Chart::new(&sets)
        .x_axis(Axis::new().bounds([10.0, 20.0]))
        .y_axis(Axis::new().bounds([100.0, 200.0]));
    assert_eq!(rows(&chart, 4, 3), ["│  █", "│█  ", "└───"]);
}

#[test]
fn y_labels_are_right_aligned_in_the_gutter() {
    let chart = Chart::new(&[]).y_axis(Axis::new().labels(&["0", "5", "10"]));
    assert_eq!(
        rows(&chart, 8, 5),
        ["10│     ", "  │     ", " 5│     ", " 0│     ", "  └─────"]
    );
}

#[test]
fn a_single_y_label_sits_on_the_bottom_row() {
    let chart = Chart::new(&[]).y_axis(Axis::new().labels(&["7"]));
    assert_eq!(rows(&chart, 4, 3), [" │  ", "7│  ", " └──"]);
}

#[test]
fn x_labels_hug_the_ends_and_center_between() {
    let chart = Chart::new(&[]).x_axis(Axis::new().labels(&["0", "5", "10"]));
    assert_eq!(
        rows(&chart, 12, 4),
        ["│          ", "│          ", "└───────────", "0     5   10"]
            .map(|row| format!("{row:<12}"))
    );
}

#[test]
fn x_labels_that_would_collide_are_skipped() {
    let chart = Chart::new(&[]).x_axis(Axis::new().labels(&["aaaa", "bbbb", "cccc"]));
    assert_eq!(rows(&chart, 9, 3), ["   │     ", "   └─────", "aaaa cccc"]);
    let chart = Chart::new(&[]).x_axis(Axis::new().labels(&["aaaa", "bb", "cccc"]));
    assert_eq!(rows(&chart, 9, 3), ["   │     ", "   └─────", "aaaa bb  "]);
}

#[test]
fn a_wide_first_x_label_widens_the_gutter() {
    let chart = Chart::new(&[]).x_axis(Axis::new().labels(&["start", "end"]));
    assert_eq!(
        rows(&chart, 12, 3),
        ["    │       ", "    └───────", "start    end"]
    );
}

#[test]
fn titles_sit_above_the_y_axis_and_under_the_x_axis() {
    let chart = Chart::new(&[])
        .y_axis(Axis::new().title("load"))
        .x_axis(Axis::new().title("time"));
    assert_eq!(
        rows(&chart, 10, 4),
        ["load      ", "│         ", "└─────────", "      time"].map(|row| format!("{row:<10}"))
    );
}

const NONE: [(f64, f64); 0] = [];

fn legend_rows(position: Option<LegendPosition>) -> Vec<String> {
    let sets = [Dataset::new(&NONE).name("ab")];
    rows(&Chart::new(&sets).legend_position(position), 22, 8)
}

#[test]
fn the_legend_lists_named_datasets_top_right() {
    assert_eq!(
        legend_rows(Some(LegendPosition::TopRight)),
        [
            "│             ╭──────╮",
            "│             │ ■ ab │",
            "│             ╰──────╯",
            "│                     ",
            "│                     ",
            "│                     ",
            "│                     ",
            "└─────────────────────",
        ]
    );
}

#[test]
fn the_legend_goes_where_it_is_told() {
    assert_eq!(
        legend_rows(Some(LegendPosition::Top))[0],
        "│      ╭──────╮       "
    );
    assert_eq!(
        legend_rows(Some(LegendPosition::TopLeft))[0],
        "│╭──────╮             "
    );
    assert_eq!(
        legend_rows(Some(LegendPosition::Left))[2],
        "│╭──────╮             "
    );
    assert_eq!(
        legend_rows(Some(LegendPosition::Right))[2],
        "│             ╭──────╮"
    );
    assert_eq!(
        legend_rows(Some(LegendPosition::BottomLeft))[6],
        "│╰──────╯             "
    );
    assert_eq!(
        legend_rows(Some(LegendPosition::BottomRight))[6],
        "│             ╰──────╯"
    );
    assert_eq!(
        legend_rows(Some(LegendPosition::Bottom))[6],
        "│      ╰──────╯       "
    );
}

#[test]
fn the_legend_can_be_hidden_or_empty() {
    assert_eq!(legend_rows(None)[0], "│                     ");
    let unnamed = [Dataset::new(&NONE)];
    assert_eq!(
        rows(&Chart::new(&unnamed), 22, 8)[0],
        "│                     "
    );
}

#[test]
fn a_legend_over_the_size_limit_is_not_drawn() {
    let sets = [Dataset::new(&NONE).name("ab")];
    let narrow =
        Chart::new(&sets).hidden_legend_constraints(Constraint::Length(7), Constraint::Length(3));
    assert_eq!(rows(&narrow, 22, 8)[0], "│                     ");
    let short =
        Chart::new(&sets).hidden_legend_constraints(Constraint::Length(8), Constraint::Length(2));
    assert_eq!(rows(&short, 22, 8)[0], "│                     ");
    let exact =
        Chart::new(&sets).hidden_legend_constraints(Constraint::Length(8), Constraint::Length(3));
    assert_eq!(rows(&exact, 22, 8)[0], "│             ╭──────╮");
    let percent = Chart::new(&sets)
        .hidden_legend_constraints(Constraint::Percentage(40), Constraint::Percentage(50));
    assert_eq!(rows(&percent, 22, 8)[0], "│             ╭──────╮");
    let ratio = Chart::new(&sets)
        .hidden_legend_constraints(Constraint::Ratio(1, 4), Constraint::Ratio(1, 2));
    assert_eq!(rows(&ratio, 22, 8)[0], "│                     ");
    let fill =
        Chart::new(&sets).hidden_legend_constraints(Constraint::Fill(1), Constraint::Fill(1));
    assert_eq!(rows(&fill, 22, 8)[0], "│             ╭──────╮");
}

#[test]
fn each_named_dataset_gets_a_legend_row() {
    let sets = [
        Dataset::new(&NONE).name("cpu"),
        Dataset::new(&NONE),
        Dataset::new(&NONE).name("memory"),
    ];
    let chart =
        Chart::new(&sets).hidden_legend_constraints(Constraint::Fill(1), Constraint::Fill(1));
    assert_eq!(
        rows(&chart, 24, 8)[..4],
        [
            "│           ╭──────────╮",
            "│           │ ■ cpu    │",
            "│           │ ■ memory │",
            "│           ╰──────────╯",
        ]
    );
}

#[test]
fn series_colors_cycle_through_theme_tokens() {
    let one = [(0.0, 0.0)];
    let order = [
        Token::Accent,
        Token::FocusRing,
        Token::Warn,
        Token::Fail,
        Token::Ok,
        Token::TextPrimary,
        Token::Accent,
    ];
    for (index, want) in order.into_iter().enumerate() {
        let sets: Vec<Dataset<'_>> = (0..=index)
            .map(|_| Dataset::new(&one).marker(Marker::Block))
            .collect();
        let (grid, _) = paint(&unit_axes(Chart::new(&sets)), 3, 2);
        assert_eq!(style_at(&grid, 1, 0).fg, token(want), "series {index}");
    }
}

#[test]
fn a_dataset_style_replaces_the_theme_color() {
    let one = [(0.0, 0.0)];
    let own = Style::new().fg(Color::Rgb(1, 2, 3));
    let sets = [Dataset::new(&one).marker(Marker::Block).style(own)];
    let (grid, _) = paint(&unit_axes(Chart::new(&sets)), 3, 2);
    assert_eq!(style_at(&grid, 1, 0), own);
}

#[test]
fn legend_swatches_use_the_series_color() {
    let sets = [Dataset::new(&NONE).name("a"), Dataset::new(&NONE).name("b")];
    let (grid, clusters) = paint(&Chart::new(&sets), 22, 10);
    assert_eq!(grid.row_display(1, &clusters), "│              │ ■ a │");
    assert_eq!(style_at(&grid, 17, 1).fg, token(Token::Accent));
    assert_eq!(style_at(&grid, 17, 2).fg, token(Token::FocusRing));
    assert_eq!(style_at(&grid, 19, 1).fg, token(Token::TextPrimary));
}

#[test]
fn axes_follow_the_widget_state() {
    let paint_state = |state| {
        let chart = Chart::new(&[])
            .state(state)
            .x_axis(Axis::new().labels(&["x"]))
            .y_axis(Axis::new().labels(&["y"]));
        paint(&chart, 5, 4).0
    };
    let normal = paint_state(WidgetState::Normal);
    assert_eq!(style_at(&normal, 1, 0).fg, token(Token::TextPrimary));
    assert!(style_at(&normal, 1, 0).attrs.dim());
    assert!(!style_at(&normal, 0, 1).attrs.dim());
    let focused = paint_state(WidgetState::Focused);
    assert_eq!(style_at(&focused, 1, 0).fg, token(Token::FocusRing));
    assert!(!style_at(&focused, 1, 0).attrs.dim());
    assert_eq!(style_at(&focused, 1, 3).fg, token(Token::TextPrimary));
    let disabled = paint_state(WidgetState::Disabled);
    assert!(style_at(&disabled, 1, 0).attrs.dim());
    assert!(style_at(&disabled, 0, 1).attrs.dim());
}

#[test]
fn disabled_dims_the_series() {
    let one = [(0.0, 0.0)];
    let sets = [Dataset::new(&one).marker(Marker::Block)];
    let chart = unit_axes(Chart::new(&sets)).state(WidgetState::Disabled);
    let (grid, _) = paint(&chart, 3, 2);
    assert!(style_at(&grid, 1, 0).attrs.dim());
    let (grid, _) = paint(&unit_axes(Chart::new(&sets)), 3, 2);
    assert!(!style_at(&grid, 1, 0).attrs.dim());
}

#[test]
fn an_axis_style_replaces_the_line_style_only() {
    let own = Style::new().fg(Color::Rgb(9, 9, 9));
    let chart = Chart::new(&[])
        .x_axis(Axis::new().labels(&["x"]).style(own))
        .y_axis(Axis::new().labels(&["y"]).style(own));
    let (grid, _) = paint(&chart, 5, 4);
    assert_eq!(style_at(&grid, 1, 0).fg, own.fg);
    assert_eq!(style_at(&grid, 1, 2).fg, own.fg);
    assert_eq!(style_at(&grid, 2, 2).fg, own.fg);
    assert_eq!(style_at(&grid, 0, 1).fg, token(Token::TextPrimary));
    assert_eq!(style_at(&grid, 1, 3).fg, token(Token::TextPrimary));
}

#[test]
fn a_label_style_replaces_the_label_and_title_style() {
    let own = Style::new().fg(Color::Rgb(7, 7, 7));
    let chart = Chart::new(&[])
        .x_axis(Axis::new().labels(&["x"]).title("tt").label_style(own))
        .y_axis(Axis::new().labels(&["y"]).title("uu").label_style(own));
    let (grid, _) = paint(&chart, 8, 6);
    assert_eq!(style_at(&grid, 0, 0).fg, own.fg);
    assert_eq!(style_at(&grid, 0, 2).fg, own.fg);
    assert_eq!(style_at(&grid, 1, 4).fg, own.fg);
    assert_eq!(style_at(&grid, 6, 5).fg, own.fg);
    assert_ne!(style_at(&grid, 1, 2).fg, own.fg);
}

#[test]
fn the_background_fills_the_area() {
    let panel = Style::new().bg(Color::Rgb(10, 20, 30));
    let one = [(0.5, 0.5)];
    let sets = [Dataset::new(&one).marker(Marker::Block)];
    let chart = unit_axes(Chart::new(&sets)).background(panel);
    let (grid, _) = paint(&chart, 4, 3);
    for (x, y) in [(0, 0), (3, 0), (0, 2), (3, 2), (1, 0), (2, 1)] {
        assert_eq!(style_at(&grid, x, y).bg, panel.bg, "background at {x},{y}");
    }
    assert_eq!(style_at(&grid, 2, 1).fg, token(Token::Accent));
}

#[test]
fn a_later_dataset_replaces_the_cell_it_shares() {
    let first = [(0.0, 1.0), (0.0, 0.0)];
    let second = [(1.0 / 3.0, 1.0)];
    let sets = [
        Dataset::new(&first).graph_type(GraphType::Scatter),
        Dataset::new(&second),
    ];
    let (grid, clusters) = paint(&unit_axes(Chart::new(&sets)), 3, 2);
    assert_eq!(grid.row_display(0, &clusters), "│⠈ ");
    assert_eq!(style_at(&grid, 1, 0).fg, token(Token::FocusRing));
}

#[test]
fn degenerate_areas_do_not_panic() {
    let data = [(0.5, 0.5)];
    let sets = [Dataset::new(&data).name("x").graph_type(GraphType::Line)];
    let chart = Chart::new(&sets)
        .x_axis(Axis::new().title("t").labels(&["a", "b"]))
        .y_axis(Axis::new().title("u").labels(&["c", "d"]));
    for width in 0..6 {
        for height in 0..6 {
            let (_, _) = paint(&chart, width, height);
        }
    }
}

#[test]
fn charts_draw_the_same_at_any_offset() {
    let data = [(0.0, 0.0), (1.0, 1.0), (0.5, 0.2)];
    let sets = [Dataset::new(&data).name("n").graph_type(GraphType::Line)];
    let chart = Chart::new(&sets)
        .x_axis(
            Axis::new()
                .title("t")
                .bounds([0.0, 1.0])
                .labels(&["0", "1"]),
        )
        .y_axis(
            Axis::new()
                .title("u")
                .bounds([0.0, 1.0])
                .labels(&["0", "1"]),
        );
    let at_origin = rows(&chart, 30, 12);
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(34, 15);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    chart.render(Rect::new(3, 2, 30, 12), &mut cx);
    let shifted: Vec<String> = (2..14)
        .map(|y| {
            grid.row_display(y, &clusters)
                .chars()
                .skip(3)
                .take(30)
                .collect()
        })
        .collect();
    assert_eq!(shifted, at_origin);
    for y in [0, 1, 14] {
        assert_eq!(grid.row_display(y, &clusters), " ".repeat(34));
    }
}

#[test]
fn non_finite_points_are_skipped_and_neighbors_join() {
    let data = [
        (0.0, 0.0),
        (0.5, f64::NAN),
        (f64::INFINITY, 0.5),
        (1.0, 1.0),
    ];
    let sets = [Dataset::new(&data)
        .marker(Marker::Block)
        .graph_type(GraphType::Line)];
    assert_eq!(
        rows(&unit_axes(Chart::new(&sets)), 6, 4),
        ["│    █", "│  ██ ", "│██   ", "└─────"]
    );
}

#[test]
fn bars_outside_the_bounds_are_dropped() {
    let data = [(0.5, 2.0), (2.0, 0.5), (-1.0, 0.5), (0.5, -1.0)];
    let sets = [Dataset::new(&data).graph_type(GraphType::Bar)];
    assert_eq!(
        rows(&unit_axes(Chart::new(&sets)), 4, 3),
        ["│   ", "│   ", "└───"]
    );
}

#[test]
fn a_legend_as_big_as_the_plot_is_drawn() {
    let sets = [Dataset::new(&NONE).name("ab")];
    let chart =
        Chart::new(&sets).hidden_legend_constraints(Constraint::Fill(1), Constraint::Fill(1));
    assert_eq!(
        rows(&chart, 9, 4),
        ["│╭──────╮", "││ ■ ab │", "│╰──────╯", "└────────"]
    );
    assert_eq!(rows(&chart, 8, 4)[0], "│       ");
    assert_eq!(rows(&chart, 9, 3)[0], "│        ");
}

#[test]
fn legend_size_limits_scale_with_the_plot() {
    let sets = [Dataset::new(&NONE).name("ab")];
    let with = |width, height| {
        rows(
            &Chart::new(&sets).hidden_legend_constraints(width, height),
            22,
            8,
        )[0]
        .clone()
    };
    let shown = "│             ╭──────╮";
    let hidden = "│                     ";
    assert_eq!(
        with(Constraint::Percentage(40), Constraint::Percentage(50)),
        shown
    );
    assert_eq!(
        with(Constraint::Percentage(30), Constraint::Percentage(50)),
        hidden
    );
    assert_eq!(
        with(Constraint::Percentage(40), Constraint::Percentage(30)),
        hidden
    );
    assert_eq!(
        with(Constraint::Ratio(2, 3), Constraint::Ratio(1, 2)),
        shown
    );
    assert_eq!(
        with(Constraint::Ratio(1, 4), Constraint::Ratio(1, 2)),
        hidden
    );
    assert_eq!(
        with(Constraint::Ratio(2, 3), Constraint::Ratio(1, 4)),
        hidden
    );
    assert_eq!(
        with(Constraint::Ratio(2, 3), Constraint::Ratio(5, 0)),
        shown
    );
}

#[test]
fn bar_heights_follow_the_lower_bound() {
    let data = [(0.0, 150.0), (0.5, 125.0), (1.0, 100.0), (1.0, 200.0)];
    let sets = [Dataset::new(&data[..3]).graph_type(GraphType::Bar)];
    let chart = Chart::new(&sets)
        .x_axis(Axis::new().bounds([0.0, 1.0]))
        .y_axis(Axis::new().bounds([100.0, 200.0]));
    assert_eq!(rows(&chart, 4, 3), ["│   ", "│█▄ ", "└───"]);
}

#[test]
fn a_legend_wider_or_taller_than_the_plot_is_never_drawn() {
    let sets = [Dataset::new(&NONE).name("ab")];
    let roomy =
        Chart::new(&sets).hidden_legend_constraints(Constraint::Length(99), Constraint::Length(99));
    assert_eq!(rows(&roomy, 8, 10)[0], "│       ");
    assert_eq!(rows(&roomy, 12, 3)[0], "│           ");
    assert_eq!(rows(&roomy, 12, 4)[0], "│   ╭──────╮");
}

#[test]
fn a_second_render_leaves_nothing_of_the_first() {
    let theme = Theme::builtin(Variant::Dark);
    let panel = Style::new().bg(Color::Rgb(10, 20, 30));
    let drawn = [(0.0, 0.0), (1.0, 1.0)];
    let first = [
        Dataset::new(&drawn).graph_type(GraphType::Line).name("old"),
        Dataset::new(&drawn).marker(Marker::Block),
    ];
    let second = [Dataset::new(&NONE)];
    let wide = Chart::new(&first)
        .background(panel)
        .hidden_legend_constraints(Constraint::Fill(1), Constraint::Fill(1))
        .x_axis(
            Axis::new()
                .title("long title")
                .labels(&["start", "middle", "finish"]),
        )
        .y_axis(Axis::new().title("another").labels(&["100", "50", "0"]));
    let narrow = Chart::new(&second).background(panel);
    for (width, height) in [(30, 10), (3, 1), (30, 2)] {
        let area = Rect::new(0, 0, width, height);
        let mut grid = Grid::new(30, 10);
        let mut clusters = ClusterTable::new();
        let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
        wide.render(Rect::new(0, 0, 30, 10), &mut cx);
        let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
        narrow.render(area, &mut cx);
        let mut fresh = Grid::new(30, 10);
        let mut fresh_clusters = ClusterTable::new();
        let mut cx = RenderCx::new(&mut fresh, &mut fresh_clusters, &theme);
        narrow.render(area, &mut cx);
        let inside = |grid: &Grid| -> Vec<u128> {
            (0..height)
                .flat_map(|y| (0..width).map(move |x| (x, y)))
                .map(|(x, y)| grid.cell_bits(x, y))
                .collect()
        };
        assert_eq!(inside(&grid), inside(&fresh), "{width}x{height}");
    }
}
