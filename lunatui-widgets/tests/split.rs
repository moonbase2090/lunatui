// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::{Cell, ClusterTable, Color, Grid, Rect, Style};
use lunatui_theme::{Theme, Variant};
use lunatui_widgets::{
    DividerId, PaneArea, PaneId, RenderCx, SplitAxis, SplitDrag, SplitLayout, SplitTree,
};

const AREA: Rect = Rect::new(0, 0, 21, 5);

fn pair(axis: SplitAxis, ratio: u16) -> (SplitTree, PaneId, PaneId) {
    let (mut tree, first) = SplitTree::new();
    let second = tree.split(first, axis, ratio).unwrap_or(first);
    (tree, first, second)
}

fn rects(layout: &SplitLayout) -> Vec<(u32, Rect)> {
    layout
        .panes
        .iter()
        .map(|area: &PaneArea| (area.pane.0, area.rect))
        .collect()
}

fn lines(layout: &SplitLayout) -> Vec<Rect> {
    layout.dividers.iter().map(|divider| divider.rect).collect()
}

#[test]
fn a_single_pane_fills_the_area() {
    let (tree, only) = SplitTree::new();
    let layout = tree.layout(AREA);
    assert_eq!(rects(&layout), [(only.0, AREA)]);
    assert_eq!(layout.dividers, []);
    assert_eq!(tree.len(), 1);
    assert!(!tree.is_empty());
}

#[test]
fn a_horizontal_split_puts_a_vertical_divider_between_two_panes() {
    let (tree, a, b) = pair(SplitAxis::Horizontal, 500);
    let layout = tree.layout(AREA);
    assert_eq!(
        rects(&layout),
        [
            (a.0, Rect::new(0, 0, 10, 5)),
            (b.0, Rect::new(11, 0, 10, 5)),
        ]
    );
    assert_eq!(lines(&layout), [Rect::new(10, 0, 1, 5)]);
    assert_eq!(layout.dividers[0].bounds, AREA);
    assert_eq!(layout.dividers[0].axis, SplitAxis::Horizontal);
}

#[test]
fn a_vertical_split_puts_a_horizontal_divider_between_two_panes() {
    let (tree, a, b) = pair(SplitAxis::Vertical, 500);
    let layout = tree.layout(Rect::new(2, 1, 9, 7));
    assert_eq!(
        rects(&layout),
        [(a.0, Rect::new(2, 1, 9, 3)), (b.0, Rect::new(2, 5, 9, 3))]
    );
    assert_eq!(lines(&layout), [Rect::new(2, 4, 9, 1)]);
}

#[test]
fn the_ratio_picks_the_first_panes_share_rounded_to_cells() {
    let widths = |ratio: u16, width: u16| {
        let (tree, _, _) = pair(SplitAxis::Horizontal, ratio);
        tree.layout(Rect::new(0, 0, width, 1))
            .panes
            .iter()
            .map(|area| area.rect.width)
            .collect::<Vec<_>>()
    };
    assert_eq!(widths(333, 11), [3, 7]);
    assert_eq!(widths(0, 11), [0, 10]);
    assert_eq!(widths(1000, 11), [10, 0]);
    assert_eq!(widths(5000, 11), [10, 0]);
    assert_eq!(widths(250, 21), [5, 15]);
    assert_eq!(widths(500, 22), [11, 10]);
    assert_eq!(widths(550, 11), [6, 4]);
    assert_eq!(widths(549, 11), [5, 5]);
}

#[test]
fn nested_splits_divide_their_own_share() {
    let (mut tree, a, b) = pair(SplitAxis::Horizontal, 500);
    let c = tree.split(b, SplitAxis::Vertical, 500).unwrap_or(b);
    let layout = tree.layout(AREA);
    assert_eq!(
        rects(&layout),
        [
            (a.0, Rect::new(0, 0, 10, 5)),
            (b.0, Rect::new(11, 0, 10, 2)),
            (c.0, Rect::new(11, 3, 10, 2)),
        ]
    );
    assert_eq!(
        lines(&layout),
        [Rect::new(10, 0, 1, 5), Rect::new(11, 2, 10, 1)]
    );
    assert_eq!(layout.dividers[0].id, DividerId(0));
    assert_eq!(layout.dividers[1].id, DividerId(1));
    assert_eq!(layout.dividers[1].bounds, Rect::new(11, 0, 10, 5));
}

#[test]
fn splitting_the_first_pane_numbers_its_divider_first_in_tree_order() {
    let (mut tree, a, b) = pair(SplitAxis::Horizontal, 500);
    let c = tree.split(a, SplitAxis::Vertical, 500).unwrap_or(a);
    assert_eq!(tree.panes(), [a, c, b]);
    let layout = tree.layout(AREA);
    assert_eq!(
        layout
            .dividers
            .iter()
            .map(|divider| (divider.id.0, divider.axis))
            .collect::<Vec<_>>(),
        [(0, SplitAxis::Horizontal), (1, SplitAxis::Vertical)]
    );
    assert_eq!(layout.dividers[1].bounds, Rect::new(0, 0, 10, 5));
}

#[test]
fn splitting_an_unknown_pane_changes_nothing() {
    let (mut tree, _, _) = pair(SplitAxis::Horizontal, 500);
    let before = tree.clone();
    assert_eq!(tree.split(PaneId(99), SplitAxis::Vertical, 500), None);
    assert_eq!(tree, before);
    assert_eq!(tree.len(), 2);
    assert!(tree.contains(PaneId(1)));
    assert!(!tree.contains(PaneId(99)));
}

#[test]
fn minimum_sizes_hold_a_pane_open() {
    let (mut tree, _, _) = pair(SplitAxis::Horizontal, 100);
    assert!(tree.set_min(DividerId(0), 6, 4));
    assert_eq!(tree.layout(AREA).panes[0].rect.width, 6);
    assert!(tree.set_ratio(DividerId(0), 1000));
    assert_eq!(tree.layout(AREA).panes[0].rect.width, 16);
    assert_eq!(tree.layout(AREA).panes[1].rect.width, 4);
    assert!(!tree.set_min(DividerId(7), 1, 1));
}

#[test]
fn minimums_that_cannot_both_fit_fall_back_to_the_ratio() {
    let (mut tree, _, _) = pair(SplitAxis::Horizontal, 500);
    tree.set_min(DividerId(0), 8, 8);
    let widths: Vec<u16> = tree
        .layout(Rect::new(0, 0, 11, 1))
        .panes
        .iter()
        .map(|area| area.rect.width)
        .collect();
    assert_eq!(widths, [5, 5]);
    tree.set_min(DividerId(0), 5, 5);
    let widths: Vec<u16> = tree
        .layout(Rect::new(0, 0, 11, 1))
        .panes
        .iter()
        .map(|area| area.rect.width)
        .collect();
    assert_eq!(widths, [5, 5]);
}

#[test]
fn closing_a_pane_gives_its_space_to_its_sibling() {
    let (mut tree, a, b) = pair(SplitAxis::Horizontal, 500);
    let c = tree.split(b, SplitAxis::Vertical, 500).unwrap_or(b);
    assert!(tree.close(b));
    assert_eq!(tree.panes(), [a, c]);
    assert_eq!(
        rects(&tree.layout(AREA)),
        [
            (a.0, Rect::new(0, 0, 10, 5)),
            (c.0, Rect::new(11, 0, 10, 5))
        ]
    );
    assert!(tree.close(a));
    assert_eq!(rects(&tree.layout(AREA)), [(c.0, AREA)]);
    assert!(!tree.close(c));
    assert_eq!(tree.len(), 1);
}

#[test]
fn closing_the_second_pane_of_a_nested_split_keeps_the_rest() {
    let (mut tree, a, b) = pair(SplitAxis::Horizontal, 500);
    let c = tree.split(a, SplitAxis::Vertical, 500).unwrap_or(a);
    assert!(tree.close(c));
    assert_eq!(tree.panes(), [a, b]);
    assert!(!tree.close(c));
    assert!(!tree.close(PaneId(99)));
    assert_eq!(tree.layout(AREA).dividers.len(), 1);
}

#[test]
fn new_panes_get_fresh_ids_after_a_close() {
    let (mut tree, a, b) = pair(SplitAxis::Horizontal, 500);
    tree.close(b);
    let c = tree.split(a, SplitAxis::Horizontal, 500).unwrap_or(a);
    assert_eq!((a.0, b.0, c.0), (0, 1, 2));
}

#[test]
fn zoom_gives_one_pane_everything_and_hides_the_dividers() {
    let (mut tree, a, b) = pair(SplitAxis::Horizontal, 500);
    tree.zoom(Some(b));
    assert_eq!(tree.zoomed(), Some(b));
    let layout = tree.layout(AREA);
    assert_eq!(rects(&layout), [(b.0, AREA)]);
    assert_eq!(layout.dividers, []);
    tree.zoom(None);
    assert_eq!(tree.zoomed(), None);
    assert_eq!(tree.layout(AREA).panes.len(), 2);
    tree.toggle_zoom(a);
    assert_eq!(tree.zoomed(), Some(a));
    tree.toggle_zoom(a);
    assert_eq!(tree.zoomed(), None);
    tree.zoom(Some(PaneId(99)));
    assert_eq!(tree.zoomed(), None);
    tree.toggle_zoom(PaneId(99));
    assert_eq!(tree.zoomed(), None);
}

#[test]
fn zoom_keeps_the_ratios_and_closing_the_zoomed_pane_ends_it() {
    let (mut tree, a, b) = pair(SplitAxis::Horizontal, 300);
    tree.zoom(Some(a));
    tree.zoom(None);
    assert_eq!(tree.ratio(DividerId(0)), Some(300));
    tree.zoom(Some(b));
    assert!(tree.close(b));
    assert_eq!(tree.zoomed(), None);
    assert_eq!(rects(&tree.layout(AREA)), [(a.0, AREA)]);
}

#[test]
fn panes_and_dividers_are_found_by_position() {
    let (tree, a, b) = pair(SplitAxis::Horizontal, 500);
    let layout = tree.layout(AREA);
    assert_eq!(layout.pane_at(0, 0), Some(a));
    assert_eq!(layout.pane_at(9, 4), Some(a));
    assert_eq!(layout.pane_at(10, 2), None);
    assert_eq!(layout.pane_at(11, 2), Some(b));
    assert_eq!(layout.pane_at(21, 2), None);
    assert_eq!(layout.pane_at(5, 5), None);
    assert_eq!(layout.divider_at(10, 0).map(|d| d.id), Some(DividerId(0)));
    assert_eq!(layout.divider_at(10, 4).map(|d| d.id), Some(DividerId(0)));
    assert_eq!(layout.divider_at(9, 2), None);
    assert_eq!(layout.divider_at(10, 5), None);
    assert_eq!(layout.rect_of(b), Some(Rect::new(11, 0, 10, 5)));
    assert_eq!(layout.rect_of(PaneId(9)), None);
}

#[test]
fn degenerate_areas_do_not_panic() {
    let (tree, _, _) = pair(SplitAxis::Horizontal, 500);
    assert_eq!(tree.layout(Rect::new(0, 0, 0, 5)).dividers, []);
    let one = tree.layout(Rect::new(3, 0, 1, 5));
    assert_eq!(lines(&one), [Rect::new(3, 0, 1, 5)]);
    assert_eq!(
        rects(&one),
        [(0, Rect::new(3, 0, 0, 5)), (1, Rect::new(4, 0, 0, 5))]
    );
    let two = tree.layout(Rect::new(0, 0, 2, 5));
    assert_eq!(
        rects(&two),
        [(0, Rect::new(0, 0, 1, 5)), (1, Rect::new(2, 0, 0, 5))]
    );
}

#[test]
fn dragging_a_divider_resizes_to_the_exact_cell_under_the_pointer() {
    for width in 2..=400_u16 {
        let area = Rect::new(4, 0, width, 3);
        for column in 4..4 + width {
            let (mut tree, _, _) = pair(SplitAxis::Horizontal, 500);
            let layout = tree.layout(area);
            let mut drag = SplitDrag::new();
            let line = layout.dividers[0].rect;
            assert!(drag.press(&tree, &layout, line.x, 1));
            drag.drag(&mut tree, column, 1);
            let resized = tree.layout(area);
            assert_eq!(
                resized.panes[0].rect.width,
                column - 4,
                "width {width} column {column}"
            );
            assert_eq!(
                resized.panes[0].rect.width + 1 + resized.panes[1].rect.width,
                width
            );
        }
    }
}

#[test]
fn dragging_past_the_edges_stops_at_the_edges() {
    let (mut tree, _, _) = pair(SplitAxis::Horizontal, 500);
    let layout = tree.layout(AREA);
    let mut drag = SplitDrag::new();
    drag.press(&tree, &layout, 10, 0);
    drag.drag(&mut tree, 200, 0);
    assert_eq!(tree.layout(AREA).panes[0].rect.width, 20);
    drag.drag(&mut tree, 0, 0);
    assert_eq!(tree.layout(AREA).panes[0].rect.width, 0);
}

#[test]
fn dragging_stops_at_the_minimum_sizes() {
    let (mut tree, _, _) = pair(SplitAxis::Horizontal, 500);
    tree.set_min(DividerId(0), 4, 6);
    let layout = tree.layout(AREA);
    let mut drag = SplitDrag::new();
    drag.press(&tree, &layout, 10, 0);
    drag.drag(&mut tree, 1, 0);
    assert_eq!(tree.layout(AREA).panes[0].rect.width, 4);
    drag.drag(&mut tree, 20, 0);
    assert_eq!(tree.layout(AREA).panes[0].rect.width, 14);
    drag.drag(&mut tree, 12, 0);
    assert_eq!(tree.layout(AREA).panes[0].rect.width, 12);
}

#[test]
fn a_vertical_divider_follows_the_row() {
    let (mut tree, _, _) = pair(SplitAxis::Vertical, 500);
    let area = Rect::new(0, 2, 10, 11);
    let layout = tree.layout(area);
    let mut drag = SplitDrag::new();
    assert!(drag.press(&tree, &layout, 3, 7));
    drag.drag(&mut tree, 0, 4);
    let resized = tree.layout(area);
    assert_eq!(resized.panes[0].rect, Rect::new(0, 2, 10, 2));
    assert_eq!(resized.dividers[0].rect, Rect::new(0, 4, 10, 1));
    assert_eq!(resized.panes[1].rect, Rect::new(0, 5, 10, 8));
}

#[test]
fn a_nested_divider_resizes_only_its_own_split() {
    let (mut tree, a, b) = pair(SplitAxis::Horizontal, 500);
    let c = tree.split(b, SplitAxis::Vertical, 500).unwrap_or(b);
    let layout = tree.layout(AREA);
    let mut drag = SplitDrag::new();
    assert!(drag.press(&tree, &layout, 15, 2));
    assert_eq!(drag.active(), Some(DividerId(1)));
    drag.drag(&mut tree, 0, 4);
    let resized = tree.layout(AREA);
    assert_eq!(resized.rect_of(a), Some(Rect::new(0, 0, 10, 5)));
    assert_eq!(resized.rect_of(b), Some(Rect::new(11, 0, 10, 4)));
    assert_eq!(resized.rect_of(c), Some(Rect::new(11, 5, 10, 0)));
}

#[test]
fn release_ends_the_drag_and_keeps_the_size() {
    let (mut tree, _, _) = pair(SplitAxis::Horizontal, 500);
    let layout = tree.layout(AREA);
    let mut drag = SplitDrag::new();
    drag.press(&tree, &layout, 10, 1);
    drag.drag(&mut tree, 6, 1);
    assert!(drag.release());
    assert!(!drag.release());
    assert_eq!(drag.active(), None);
    drag.drag(&mut tree, 15, 1);
    assert_eq!(tree.layout(AREA).panes[0].rect.width, 6);
}

#[test]
fn cancel_puts_the_split_back() {
    let (mut tree, _, _) = pair(SplitAxis::Horizontal, 400);
    let layout = tree.layout(AREA);
    let mut drag = SplitDrag::new();
    drag.press(&tree, &layout, 8, 1);
    drag.drag(&mut tree, 17, 1);
    assert_ne!(tree.ratio(DividerId(0)), Some(400));
    drag.cancel(&mut tree);
    assert_eq!(tree.ratio(DividerId(0)), Some(400));
    assert_eq!(drag.active(), None);
    drag.cancel(&mut tree);
    assert_eq!(tree.ratio(DividerId(0)), Some(400));
}

#[test]
fn a_press_off_every_divider_starts_nothing() {
    let (mut tree, _, _) = pair(SplitAxis::Horizontal, 500);
    let layout = tree.layout(AREA);
    let mut drag = SplitDrag::new();
    assert!(!drag.press(&tree, &layout, 9, 1));
    assert!(!drag.press(&tree, &layout, 11, 1));
    assert!(!drag.press(&tree, &layout, 10, 5));
    drag.drag(&mut tree, 3, 1);
    assert_eq!(tree.ratio(DividerId(0)), Some(500));
    assert!(!drag.release());
}

#[test]
fn a_new_press_replaces_a_drag_in_progress() {
    let (tree, _, _) = pair(SplitAxis::Horizontal, 500);
    let layout = tree.layout(AREA);
    let mut drag = SplitDrag::new();
    assert!(drag.press(&tree, &layout, 10, 1));
    assert!(!drag.press(&tree, &layout, 3, 1));
    assert_eq!(drag.active(), None);
}

#[test]
fn ratios_and_set_ratio_clamp_and_report_unknown_dividers() {
    let (mut tree, _, _) = pair(SplitAxis::Horizontal, 500);
    assert_eq!(tree.ratio(DividerId(0)), Some(500));
    assert_eq!(tree.ratio(DividerId(1)), None);
    assert!(tree.set_ratio(DividerId(0), 9000));
    assert_eq!(tree.ratio(DividerId(0)), Some(1000));
    assert!(!tree.set_ratio(DividerId(3), 10));
    assert_eq!(SplitTree::default().len(), 1);
}

#[test]
fn dividers_draw_as_lines_and_the_active_one_in_its_own_style() {
    let (mut tree, _, b) = pair(SplitAxis::Horizontal, 500);
    tree.split(b, SplitAxis::Vertical, 500);
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(21, 5);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    let plain = Style::new().fg(Color::Rgb(1, 2, 3));
    let bold = Style::new().fg(Color::Rgb(9, 9, 9));
    tree.layout(AREA)
        .render_dividers(plain, Some(DividerId(1)), bold, &mut cx);
    let rows: Vec<String> = (0..5).map(|y| grid.row_display(y, &clusters)).collect();
    assert_eq!(
        rows,
        [
            "          │          ",
            "          │          ",
            "          │━━━━━━━━━━",
            "          │          ",
            "          │          ",
        ]
    );
    let fg = |x, y| grid.get(x, y).map(Cell::style).unwrap_or_default().fg;
    assert_eq!(fg(10, 0), plain.fg);
    assert_eq!(fg(15, 2), bold.fg);

    let mut flat = Grid::new(21, 5);
    let mut cx = RenderCx::new(&mut flat, &mut clusters, &theme);
    tree.layout(AREA)
        .render_dividers(plain, Some(DividerId(0)), bold, &mut cx);
    assert_eq!(flat.row_display(2, &clusters), "          ┃──────────");
    let fg = |x, y| flat.get(x, y).map(Cell::style).unwrap_or_default().fg;
    assert_eq!(fg(10, 2), bold.fg);
    assert_eq!(fg(15, 2), plain.fg);
}

#[test]
fn a_minimum_that_exactly_fills_the_room_pins_the_split() {
    let (mut tree, _, _) = pair(SplitAxis::Horizontal, 900);
    tree.set_min(DividerId(0), 5, 5);
    let widths: Vec<u16> = tree
        .layout(Rect::new(0, 0, 11, 1))
        .panes
        .iter()
        .map(|area| area.rect.width)
        .collect();
    assert_eq!(widths, [5, 5]);
}

#[test]
fn dragging_in_a_one_cell_area_centers_the_ratio() {
    let (mut tree, _, _) = pair(SplitAxis::Horizontal, 200);
    let area = Rect::new(0, 0, 1, 3);
    let layout = tree.layout(area);
    let mut drag = SplitDrag::new();
    assert!(drag.press(&tree, &layout, 0, 1));
    drag.drag(&mut tree, 0, 1);
    assert_eq!(tree.ratio(DividerId(0)), Some(500));
}
