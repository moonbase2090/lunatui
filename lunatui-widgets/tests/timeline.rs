// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::{ClusterTable, Color, Grid, Rect, Size, Style, UnderlineStyle};
use lunatui_theme::{Theme, Token, Variant};
use lunatui_widgets::{
    RenderCx, Timeline, TimelineItem, TimelineItemId, TimelineLane, TimelineStatus, TimelineTick,
    TimelineView, Widget, WidgetState,
};

fn draw(timeline: &Timeline<'_>, area: Rect, width: u16, height: u16) -> (Grid, ClusterTable) {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(width, height);
    let mut clusters = ClusterTable::new();
    grid.fill(
        Rect::new(0, 0, width, height),
        Style::new(),
        '#',
        &mut clusters,
    );
    timeline.render(area, &mut RenderCx::new(&mut grid, &mut clusters, &theme));
    (grid, clusters)
}

fn style(grid: &Grid, x: u16, y: u16) -> Style {
    grid.get(x, y)
        .map(lunatui_core::Cell::style)
        .unwrap_or_default()
}

fn color(token: Token) -> Color {
    Theme::builtin(Variant::Dark).resolve(token)
}

#[test]
fn spans_fill_their_columns_with_the_status_color_and_label() {
    let items = [TimelineItem::span(10, 40)
        .label("build")
        .status(TimelineStatus::Done)];
    let lanes = [TimelineLane::new("", &items)];
    let timeline = Timeline::new(&lanes).view(TimelineView::new(0, 100));
    let (grid, clusters) = draw(&timeline, Rect::new(0, 0, 20, 1), 20, 1);
    assert_eq!(grid.row_display(0, &clusters), "  build             ");
    for x in 2..8 {
        assert_eq!(style(&grid, x, 0).bg, color(Token::Ok), "column {x}");
    }
    assert_eq!(style(&grid, 2, 0).fg, color(Token::Surface0));
    assert_eq!(style(&grid, 1, 0).bg, color(Token::Surface0));
    assert_eq!(style(&grid, 8, 0).bg, color(Token::Surface0));
}

#[test]
fn each_status_uses_its_semantic_token() {
    let items = [
        TimelineItem::span(0, 10).progress(100),
        TimelineItem::span(10, 20).status(TimelineStatus::Done),
        TimelineItem::span(20, 30).status(TimelineStatus::AtRisk),
        TimelineItem::span(30, 40).status(TimelineStatus::Blocked),
    ];
    let lanes = [TimelineLane::new("", &items)];
    let timeline = Timeline::new(&lanes).view(TimelineView::new(0, 40));
    let (grid, _) = draw(&timeline, Rect::new(0, 0, 8, 1), 8, 1);
    for (x, token) in [
        (0, Token::Accent),
        (2, Token::Ok),
        (4, Token::Warn),
        (6, Token::Fail),
    ] {
        assert_eq!(style(&grid, x, 0).bg, color(token));
        assert_eq!(style(&grid, x + 1, 0).bg, color(token));
    }
}

#[test]
fn progress_splits_a_span_into_finished_and_track_columns() {
    let items = [TimelineItem::span(0, 50).label("migrate").progress(40)];
    let lanes = [TimelineLane::new("", &items)];
    let timeline = Timeline::new(&lanes).view(TimelineView::new(0, 100));
    let (grid, clusters) = draw(&timeline, Rect::new(0, 0, 20, 1), 20, 1);
    assert_eq!(grid.row_display(0, &clusters), "migrate             ");
    for x in 0..4 {
        assert_eq!(style(&grid, x, 0).bg, color(Token::Accent), "column {x}");
        assert_eq!(style(&grid, x, 0).fg, color(Token::Surface0), "column {x}");
    }
    for x in 4..10 {
        assert_eq!(style(&grid, x, 0).bg, color(Token::Surface1), "column {x}");
    }
    assert_eq!(style(&grid, 5, 0).fg, color(Token::TextPrimary));
    assert_eq!(style(&grid, 10, 0).bg, color(Token::Surface0));
}

#[test]
fn planned_spans_without_progress_are_all_track_and_others_are_solid() {
    let items = [
        TimelineItem::span(0, 20).label("later"),
        TimelineItem::span(20, 40).status(TimelineStatus::AtRisk),
    ];
    let lanes = [TimelineLane::new("", &items)];
    let timeline = Timeline::new(&lanes).view(TimelineView::new(0, 40));
    let (grid, clusters) = draw(&timeline, Rect::new(0, 0, 8, 1), 8, 1);
    assert_eq!(grid.row_display(0, &clusters), "late    ");
    for x in 0..4 {
        assert_eq!(style(&grid, x, 0).bg, color(Token::Surface1), "column {x}");
        assert_eq!(
            style(&grid, x, 0).fg,
            color(Token::TextPrimary),
            "column {x}"
        );
    }
    for x in 4..8 {
        assert_eq!(style(&grid, x, 0).bg, color(Token::Warn), "column {x}");
    }
}

#[test]
fn progress_over_one_hundred_finishes_the_span() {
    let items = [TimelineItem::span(0, 50).progress(250)];
    let lanes = [TimelineLane::new("", &items)];
    let timeline = Timeline::new(&lanes).view(TimelineView::new(0, 100));
    let (grid, _) = draw(&timeline, Rect::new(0, 0, 20, 1), 20, 1);
    assert_eq!(style(&grid, 9, 0).bg, color(Token::Accent));
}

#[test]
fn short_spans_keep_one_cell_and_partial_columns_round_outward() {
    let items = [
        TimelineItem::span(7, 8).progress(100),
        TimelineItem::span(12, 21).progress(100),
    ];
    let lanes = [TimelineLane::new("", &items)];
    let timeline = Timeline::new(&lanes).view(TimelineView::new(0, 100));
    let (grid, _) = draw(&timeline, Rect::new(0, 0, 20, 1), 20, 1);
    let painted: Vec<u16> = (0..20)
        .filter(|&x| style(&grid, x, 0).bg == color(Token::Accent))
        .collect();
    assert_eq!(painted, [1, 2, 3, 4]);
}

#[test]
fn milestones_draw_a_diamond_with_the_label_to_its_right() {
    let items = [TimelineItem::milestone(30)
        .label("GA")
        .status(TimelineStatus::AtRisk)];
    let lanes = [TimelineLane::new("", &items)];
    let timeline = Timeline::new(&lanes).view(TimelineView::new(0, 100));
    let (grid, clusters) = draw(&timeline, Rect::new(0, 0, 20, 1), 20, 1);
    assert_eq!(grid.row_display(0, &clusters), "      ◆ GA          ");
    assert_eq!(style(&grid, 6, 0).fg, color(Token::Warn));
    assert_eq!(style(&grid, 6, 0).bg, color(Token::Surface0));
    assert_eq!(style(&grid, 8, 0).fg, color(Token::TextPrimary));
}

#[test]
fn milestone_labels_clip_at_the_right_edge() {
    let items = [TimelineItem::milestone(80).label("release candidate")];
    let lanes = [TimelineLane::new("", &items)];
    let timeline = Timeline::new(&lanes).view(TimelineView::new(0, 100));
    let (grid, clusters) = draw(&timeline, Rect::new(0, 0, 20, 1), 20, 1);
    assert_eq!(grid.row_display(0, &clusters), "                ◆ re");
}

#[test]
fn lane_labels_take_a_gutter_and_clip_inside_it() {
    let first = [TimelineItem::span(0, 100).progress(100)];
    let lanes = [
        TimelineLane::new("api", &first),
        TimelineLane::new("db", &[]),
    ];
    let timeline = Timeline::new(&lanes).view(TimelineView::new(0, 100));
    let (grid, clusters) = draw(&timeline, Rect::new(0, 0, 14, 2), 14, 2);
    assert_eq!(grid.row_display(0, &clusters), "api           ");
    assert_eq!(grid.row_display(1, &clusters), "db            ");
    assert_eq!(style(&grid, 3, 0).bg, color(Token::Surface0));
    assert_eq!(style(&grid, 4, 0).bg, color(Token::Accent));
    assert_eq!(style(&grid, 13, 0).bg, color(Token::Accent));
    assert_eq!(timeline.x_at(Rect::new(0, 0, 14, 2), 0), Some(4));
}

#[test]
fn ticks_draw_labels_at_their_time_and_marks_on_the_rule() {
    let items = [TimelineItem::span(0, 10).progress(100)];
    let lanes = [TimelineLane::new("", &items)];
    let ticks = [
        TimelineTick::new(0, "Mon"),
        TimelineTick::new(25, "Tuesday"),
        TimelineTick::new(50, "Wed"),
        TimelineTick::new(150, "never"),
    ];
    let timeline = Timeline::new(&lanes)
        .view(TimelineView::new(0, 100))
        .ticks(&ticks);
    let (grid, clusters) = draw(&timeline, Rect::new(0, 0, 20, 3), 20, 3);
    assert_eq!(grid.row_display(0, &clusters), "Mon  Tues Wed       ");
    assert_eq!(grid.row_display(1, &clusters), "┬────┬────┬─────────");
    assert_eq!(style(&grid, 0, 2).bg, color(Token::Accent));
}

#[test]
fn now_marks_the_rule_and_empty_lane_cells_but_not_spans() {
    let busy = [TimelineItem::span(0, 100).progress(100)];
    let lanes = [TimelineLane::new("", &[]), TimelineLane::new("", &busy)];
    let ticks = [TimelineTick::new(0, "0")];
    let timeline = Timeline::new(&lanes)
        .view(TimelineView::new(0, 100))
        .ticks(&ticks)
        .now(Some(60));
    let (grid, clusters) = draw(&timeline, Rect::new(0, 0, 20, 4), 20, 4);
    assert_eq!(grid.row_display(1, &clusters), "┬───────────▼───────");
    assert!(style(&grid, 12, 1).attrs.bold());
    assert_eq!(grid.row_display(2, &clusters), "            ┊       ");
    assert_eq!(grid.row_display(3, &clusters), "                    ");
    assert_eq!(style(&grid, 12, 3).bg, color(Token::Accent));
}

#[test]
fn now_outside_the_view_draws_nothing() {
    let lanes = [TimelineLane::new("", &[])];
    let timeline = Timeline::new(&lanes)
        .view(TimelineView::new(0, 100))
        .now(Some(100));
    let (grid, clusters) = draw(&timeline, Rect::new(0, 0, 10, 1), 10, 1);
    assert_eq!(grid.row_display(0, &clusters), "          ");
}

#[test]
fn panning_moves_items_and_clips_spans_at_the_view_edges() {
    let items = [
        TimelineItem::span(0, 30).label("early").progress(100),
        TimelineItem::milestone(90),
    ];
    let lanes = [TimelineLane::new("", &items)];
    let view = TimelineView::new(0, 100).pan(20);
    let timeline = Timeline::new(&lanes).view(view);
    let (grid, clusters) = draw(&timeline, Rect::new(0, 0, 20, 1), 20, 1);
    assert_eq!(grid.row_display(0, &clusters), "ea            ◆     ");
    assert_eq!(style(&grid, 1, 0).bg, color(Token::Accent));
    assert_eq!(style(&grid, 2, 0).bg, color(Token::Surface0));
}

#[test]
fn items_entirely_outside_the_view_are_hidden() {
    let items = [TimelineItem::span(-50, 0), TimelineItem::milestone(100)];
    let lanes = [TimelineLane::new("", &items)];
    let timeline = Timeline::new(&lanes).view(TimelineView::new(0, 100));
    let (grid, clusters) = draw(&timeline, Rect::new(0, 0, 10, 1), 10, 1);
    assert_eq!(grid.row_display(0, &clusters), "          ");
    assert_eq!(timeline.item_at(Rect::new(0, 0, 10, 1), 0, 0), None);
}

#[test]
fn selection_underlines_the_item_and_focus_colors_its_lane_label() {
    let items = [TimelineItem::span(0, 50), TimelineItem::milestone(80)];
    let lanes = [
        TimelineLane::new("ops", &items),
        TimelineLane::new("web", &[]),
    ];
    let selected = Some(TimelineItemId { lane: 0, item: 0 });
    let area = Rect::new(0, 0, 24, 2);
    let timeline = Timeline::new(&lanes)
        .view(TimelineView::new(0, 100))
        .selected(selected)
        .state(WidgetState::Focused);
    let (grid, _) = draw(&timeline, area, 24, 2);
    let span = style(&grid, 4, 0);
    assert!(span.attrs.bold());
    assert_eq!(span.attrs.underline(), UnderlineStyle::Single);
    assert_eq!(style(&grid, 20, 0).attrs.underline(), UnderlineStyle::None);
    assert_eq!(style(&grid, 0, 0).fg, color(Token::FocusRing));
    assert!(style(&grid, 0, 0).attrs.bold());
    assert_eq!(style(&grid, 0, 1).fg, color(Token::TextPrimary));

    let unfocused = Timeline::new(&lanes)
        .view(TimelineView::new(0, 100))
        .selected(selected);
    let (grid, _) = draw(&unfocused, area, 24, 2);
    assert_eq!(style(&grid, 4, 0).attrs.underline(), UnderlineStyle::Single);
    assert_eq!(style(&grid, 0, 0).fg, color(Token::TextPrimary));
}

#[test]
fn a_selected_milestone_is_underlined() {
    let items = [TimelineItem::milestone(50).label("ship")];
    let lanes = [TimelineLane::new("", &items)];
    let timeline = Timeline::new(&lanes)
        .view(TimelineView::new(0, 100))
        .selected(Some(TimelineItemId { lane: 0, item: 0 }));
    let (grid, _) = draw(&timeline, Rect::new(0, 0, 20, 1), 20, 1);
    assert_eq!(
        style(&grid, 10, 0).attrs.underline(),
        UnderlineStyle::Single
    );
    assert_eq!(
        style(&grid, 12, 0).attrs.underline(),
        UnderlineStyle::Single
    );
}

#[test]
fn disabled_timelines_use_dim_swatches_and_drop_selection() {
    let items = [TimelineItem::span(0, 50).status(TimelineStatus::Blocked)];
    let lanes = [TimelineLane::new("", &items)];
    let timeline = Timeline::new(&lanes)
        .view(TimelineView::new(0, 100))
        .selected(Some(TimelineItemId { lane: 0, item: 0 }))
        .state(WidgetState::Disabled);
    let (grid, clusters) = draw(&timeline, Rect::new(0, 0, 20, 1), 20, 1);
    assert_eq!(grid.row_display(0, &clusters), "░░░░░░░░░░          ");
    let swatch = style(&grid, 3, 0);
    assert_eq!(swatch.fg, color(Token::Fail));
    assert_eq!(swatch.bg, color(Token::Surface0));
    assert!(swatch.attrs.dim());
    assert_eq!(swatch.attrs.underline(), UnderlineStyle::None);
}

#[test]
fn item_at_returns_the_topmost_item_and_milestones_only_on_their_glyph() {
    let items = [
        TimelineItem::span(0, 60),
        TimelineItem::span(40, 80),
        TimelineItem::milestone(90).label("done"),
    ];
    let lanes = [TimelineLane::new("x", &items)];
    let area = Rect::new(0, 0, 22, 1);
    let timeline = Timeline::new(&lanes).view(TimelineView::new(0, 100));
    let id = |item| Some(TimelineItemId { lane: 0, item });
    assert_eq!(timeline.item_at(area, 2, 0), id(0));
    assert_eq!(timeline.item_at(area, 11, 0), id(1));
    assert_eq!(timeline.item_at(area, 20, 0), id(2));
    assert_eq!(timeline.item_at(area, 21, 0), None);
    assert_eq!(timeline.item_at(area, 0, 0), None);
    assert_eq!(timeline.item_at(area, 2, 1), None);
}

#[test]
fn x_at_and_time_at_share_the_render_geometry() {
    let lanes = [TimelineLane::new("ab", &[])];
    let area = Rect::new(5, 0, 23, 1);
    let timeline = Timeline::new(&lanes).view(TimelineView::new(100, 300));
    assert_eq!(timeline.x_at(area, 100), Some(8));
    assert_eq!(timeline.x_at(area, 299), Some(27));
    assert_eq!(timeline.x_at(area, 300), None);
    assert_eq!(timeline.x_at(area, 99), None);
    assert_eq!(timeline.time_at(area, 8), Some(100));
    assert_eq!(timeline.time_at(area, 18), Some(200));
    assert_eq!(timeline.time_at(area, 7), None);
    assert_eq!(timeline.time_at(area, 28), None);

    let zoomed = Timeline::new(&lanes).view(TimelineView::new(0, 4));
    assert_eq!(zoomed.time_at(area, 12), Some(0));
    assert_eq!(zoomed.time_at(area, 13), Some(1));
    assert_eq!(zoomed.time_at(area, 17), Some(1));
    assert_eq!(zoomed.x_at(area, 3), Some(23));
}

#[test]
fn lane_offset_scrolls_lanes_vertically() {
    let a = [TimelineItem::span(0, 10)];
    let b = [TimelineItem::span(0, 10).status(TimelineStatus::Done)];
    let lanes = [TimelineLane::new("a", &a), TimelineLane::new("b", &b)];
    let area = Rect::new(0, 0, 12, 2);
    let timeline = Timeline::new(&lanes)
        .view(TimelineView::new(0, 100))
        .lane_offset(1);
    let (grid, clusters) = draw(&timeline, area, 12, 2);
    assert_eq!(grid.row_display(0, &clusters), "b           ");
    assert_eq!(style(&grid, 2, 0).bg, color(Token::Ok));
    assert_eq!(grid.row_display(1, &clusters), "            ");
    assert_eq!(
        timeline.item_at(area, 2, 0),
        Some(TimelineItemId { lane: 1, item: 0 })
    );
    assert_eq!(timeline.measure(Size::new(40, 40)), Size::new(40, 1));
}

#[test]
fn rendering_stays_inside_the_area() {
    let items = [TimelineItem::span(0, 100).label("everything")];
    let lanes = [TimelineLane::new("lane", &items)];
    let ticks = [TimelineTick::new(0, "start")];
    let timeline = Timeline::new(&lanes)
        .view(TimelineView::new(0, 100))
        .ticks(&ticks)
        .now(Some(50));
    let (grid, clusters) = draw(&timeline, Rect::new(2, 1, 12, 3), 16, 5);
    assert_eq!(grid.row_display(0, &clusters), "################");
    assert_eq!(grid.row_display(4, &clusters), "################");
    for y in 1..4 {
        let row = grid.row_display(y, &clusters);
        assert!(row.starts_with("##") && row.ends_with("##"), "{row:?}");
    }
}

#[test]
fn bounds_cover_every_item() {
    let a = [TimelineItem::span(40, 10), TimelineItem::milestone(-5)];
    let b = [TimelineItem::span(60, 90)];
    let lanes = [TimelineLane::new("", &a), TimelineLane::new("", &b)];
    assert_eq!(Timeline::new(&lanes).bounds(), Some((-5, 90)));
    assert_eq!(Timeline::new(&[TimelineLane::new("", &[])]).bounds(), None);
}

#[test]
fn measure_reports_lanes_plus_header_within_the_available_size() {
    let lanes = [TimelineLane::new("", &[]), TimelineLane::new("", &[])];
    let ticks = [TimelineTick::new(0, "0")];
    assert_eq!(
        Timeline::new(&lanes).measure(Size::new(30, 10)),
        Size::new(30, 2)
    );
    assert_eq!(
        Timeline::new(&lanes)
            .ticks(&ticks)
            .measure(Size::new(30, 3)),
        Size::new(30, 3)
    );
    assert_eq!(
        Timeline::new(&[]).measure(Size::new(30, 10)),
        Size::new(0, 0)
    );
}

#[test]
fn view_pans_zooms_and_reveals() {
    let view = TimelineView::new(100, 0);
    assert_eq!((view.start(), view.end(), view.len()), (0, 100, 100));
    assert_eq!(view.pan(-40), TimelineView::new(-40, 60));
    assert_eq!(view.zoom_in(), TimelineView::new(25, 75));
    assert_eq!(view.zoom_out(), TimelineView::new(-50, 150));
    assert_eq!(TimelineView::new(0, 1).zoom_in(), TimelineView::new(0, 1));
    assert_eq!(view.reveal(20, 30), view);
    assert_eq!(view.reveal(90, 130), TimelineView::new(30, 130));
    assert_eq!(view.reveal(-30, -10), TimelineView::new(-30, 70));
    assert_eq!(view.reveal(500, 800), TimelineView::new(500, 600));
    assert_eq!(TimelineView::new(7, 7).len(), 1);
}

#[test]
fn view_reveals_a_milestone_at_the_exclusive_end() {
    let view = TimelineView::new(20, 120);
    let revealed = view.reveal(120, 120);
    assert_eq!(revealed, TimelineView::new(21, 121));
    assert_eq!(
        Timeline::new(&[])
            .view(revealed)
            .x_at(Rect::new(0, 0, 20, 1), 120),
        Some(19)
    );
    assert_eq!(view.reveal(20, 20), view);
    assert_eq!(view.reveal(70, 70), view);
    assert_eq!(view.reveal(19, 19), TimelineView::new(19, 119));
    assert_eq!(view.reveal(121, 121), TimelineView::new(22, 122));
}

#[test]
fn view_arithmetic_saturates_at_the_time_range() {
    let view = TimelineView::new(i64::MAX - 10, i64::MAX);
    assert_eq!(view.pan(100), view);
    assert_eq!(
        TimelineView::new(i64::MIN, i64::MIN + 10).pan(-5),
        TimelineView::new(i64::MIN, i64::MIN + 10)
    );
    let all = TimelineView::new(i64::MIN, i64::MAX);
    assert_eq!(all.zoom_out(), all);
    assert_eq!(TimelineView::new(i64::MAX, i64::MAX).len(), 1);
    let items = [TimelineItem::span(i64::MIN, i64::MAX).progress(100)];
    let lanes = [TimelineLane::new("", &items)];
    let (grid, _) = draw(
        &Timeline::new(&lanes).view(all),
        Rect::new(0, 0, 10, 1),
        10,
        1,
    );
    assert_eq!(style(&grid, 9, 0).bg, color(Token::Accent));
}
