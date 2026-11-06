// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::{ClusterTable, Color, Grid, Rect, Style};
use lunatui_theme::{Theme, Variant};
use lunatui_widgets::{ItemArea, RenderCx, Widget, WidgetList, WidgetListState};

const HEIGHTS: [u16; 5] = [1, 3, 2, 4, 1];
const HIGHLIGHT: Color = Color::Rgb(42, 35, 87);

fn height(index: usize, _width: u16) -> u16 {
    HEIGHTS[index]
}

fn draw(index: usize, area: Rect, _selected: bool, cx: &mut RenderCx<'_>) {
    for row in 0..area.height {
        let text = format!("{index}.{row}");
        cx.put_str(area.x, area.y + row, &text, Style::new());
    }
}

type Height = fn(usize, u16) -> u16;
type Draw = fn(usize, Rect, bool, &mut RenderCx<'_>);

fn bg(grid: &Grid, x: u16, y: u16) -> Color {
    grid.get(x, y)
        .map(|cell| cell.style().bg)
        .unwrap_or_default()
}

fn list(state: WidgetListState) -> WidgetList<Height, Draw> {
    WidgetList::new(HEIGHTS.len(), height as Height, draw as Draw)
        .highlight_style(Style::new().bg(HIGHLIGHT))
        .state(state)
}

fn render(widget: &impl Widget, width: u16, rows: u16) -> (Grid, ClusterTable) {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(width, rows);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    cx.put_str(0, 0, &"x".repeat(usize::from(width)), Style::new());
    widget.render(Rect::new(0, 0, width, rows), &mut cx);
    (grid, clusters)
}

fn text(grid: &Grid, clusters: &ClusterTable) -> Vec<String> {
    (0..grid.height())
        .map(|y| grid.row_display(y, clusters).trim_end().to_owned())
        .collect()
}

fn state(selected: Option<usize>, offset: usize, skip: u16) -> WidgetListState {
    WidgetListState {
        selected,
        offset,
        skip,
    }
}

#[test]
fn items_of_different_heights_stack_and_clear_stale_cells() {
    let (grid, clusters) = render(&list(WidgetListState::new()), 6, 12);
    assert_eq!(
        text(&grid, &clusters),
        [
            "0.0", "1.0", "1.1", "1.2", "2.0", "2.1", "3.0", "3.1", "3.2", "3.3", "4.0", ""
        ]
    );
}

#[test]
fn a_partly_scrolled_first_item_shows_only_its_lower_rows() {
    let (grid, clusters) = render(&list(state(None, 1, 2)), 6, 4);
    assert_eq!(text(&grid, &clusters), ["1.2", "2.0", "2.1", "3.0"]);
}

#[test]
fn the_last_visible_item_is_cut_at_the_area_bottom() {
    let (grid, clusters) = render(&list(WidgetListState::new()), 6, 7);
    assert_eq!(
        text(&grid, &clusters),
        ["0.0", "1.0", "1.1", "1.2", "2.0", "2.1", "3.0"]
    );
}

#[test]
fn selecting_an_item_below_the_area_scrolls_it_fully_into_view() {
    let widget = list(state(Some(3), 0, 0));
    let scrolled = widget.scrolled(Rect::new(0, 0, 6, 7));
    assert_eq!((scrolled.offset, scrolled.skip), (1, 2));
    let (grid, clusters) = render(&widget, 6, 7);
    assert_eq!(
        text(&grid, &clusters),
        ["1.2", "2.0", "2.1", "3.0", "3.1", "3.2", "3.3"]
    );
}

#[test]
fn selecting_an_item_above_the_area_scrolls_to_its_first_row() {
    let scrolled = list(state(Some(1), 3, 1)).scrolled(Rect::new(0, 0, 6, 4));
    assert_eq!(
        (scrolled.offset, scrolled.skip, scrolled.selected),
        (1, 0, Some(1))
    );
}

#[test]
fn a_selected_item_already_in_view_does_not_move_the_scroll() {
    let scrolled = list(state(Some(2), 1, 1)).scrolled(Rect::new(0, 0, 6, 6));
    assert_eq!((scrolled.offset, scrolled.skip), (1, 1));
}

#[test]
fn an_item_taller_than_the_area_shows_its_top_rows() {
    let widget = list(state(Some(3), 0, 0));
    let scrolled = widget.scrolled(Rect::new(0, 0, 6, 3));
    assert_eq!((scrolled.offset, scrolled.skip), (3, 0));
    let (grid, clusters) = render(&widget, 6, 3);
    assert_eq!(text(&grid, &clusters), ["3.0", "3.1", "3.2"]);
}

#[test]
fn scroll_padding_reveals_the_next_item_before_the_selection_reaches_the_edge() {
    let widget = list(state(Some(2), 0, 0)).scroll_padding(2);
    let scrolled = widget.scrolled(Rect::new(0, 0, 6, 6));
    assert_eq!((scrolled.offset, scrolled.skip), (1, 1));
    let (grid, clusters) = render(&widget, 6, 6);
    assert_eq!(
        text(&grid, &clusters),
        ["1.1", "1.2", "2.0", "2.1", "3.0", "3.1"]
    );
    let plain = list(state(Some(2), 0, 0)).scrolled(Rect::new(0, 0, 6, 6));
    assert_eq!((plain.offset, plain.skip), (0, 0));
}

#[test]
fn the_selected_item_gets_the_highlight_behind_every_one_of_its_rows() {
    let (grid, _) = render(&list(state(Some(1), 0, 0)), 6, 12);
    for y in 1..=3 {
        for x in 0..6 {
            assert_eq!(bg(&grid, x, y), HIGHLIGHT, "({x},{y})");
        }
    }
    assert_ne!(bg(&grid, 5, 0), HIGHLIGHT);
    assert_ne!(bg(&grid, 5, 4), HIGHLIGHT);
}

#[test]
fn a_selected_item_taller_than_the_area_is_highlighted_on_every_visible_row() {
    let (grid, _) = render(&list(state(Some(3), 0, 0)), 6, 3);
    for y in 0..3 {
        for x in 0..6 {
            assert_eq!(bg(&grid, x, y), HIGHLIGHT, "({x},{y})");
        }
    }
}

#[test]
fn an_unselected_item_cut_by_the_area_bottom_gets_no_highlight() {
    let (grid, _) = render(&list(state(Some(1), 0, 0)), 6, 5);
    assert_eq!(bg(&grid, 5, 3), HIGHLIGHT);
    assert_ne!(bg(&grid, 5, 4), HIGHLIGHT);
}

#[test]
fn item_areas_report_the_visible_rect_skip_and_full_height() {
    let areas = list(state(None, 1, 2)).item_areas(Rect::new(2, 5, 6, 4));
    assert_eq!(
        areas,
        [
            ItemArea {
                index: 1,
                rect: Rect::new(2, 5, 6, 1),
                skip: 2,
                height: 3
            },
            ItemArea {
                index: 2,
                rect: Rect::new(2, 6, 6, 2),
                skip: 0,
                height: 2
            },
            ItemArea {
                index: 3,
                rect: Rect::new(2, 8, 6, 1),
                skip: 0,
                height: 4
            },
        ]
    );
}

#[test]
fn zero_height_items_take_no_rows() {
    let widget = WidgetList::new(
        3,
        |index, _| u16::from(index != 1),
        |index, area, _, cx: &mut RenderCx<'_>| {
            cx.put_str(area.x, area.y, &index.to_string(), Style::new());
        },
    );
    let (grid, clusters) = render(&widget, 3, 3);
    assert_eq!(text(&grid, &clusters), ["0", "2", ""]);
}

#[test]
fn empty_lists_and_empty_areas_draw_nothing() {
    let empty = WidgetList::new(0, |_, _| 1, |_, _, _, _: &mut RenderCx<'_>| unreachable!());
    let (grid, clusters) = render(&empty, 4, 2);
    assert_eq!(text(&grid, &clusters), ["", ""]);
    let (grid, _) = render(&list(WidgetListState::new()), 0, 0);
    assert_eq!(grid.height(), 0);
}

#[test]
fn the_height_closure_sees_the_area_width() {
    let widget = WidgetList::new(
        1,
        |_, width| if width < 10 { 2 } else { 1 },
        |_, area, _, cx: &mut RenderCx<'_>| {
            for row in 0..area.height {
                cx.put_str(area.x, area.y + row, "w", Style::new());
            }
        },
    );
    let (narrow, clusters) = render(&widget, 5, 3);
    assert_eq!(text(&narrow, &clusters), ["w", "w", ""]);
    let (wide, clusters) = render(&widget, 12, 3);
    assert_eq!(text(&wide, &clusters), ["w", "", ""]);
}

#[test]
fn state_stepping_clamps_at_the_ends_and_starts_from_the_edges() {
    let mut state = WidgetListState::new();
    state.select_next(3);
    assert_eq!(state.selected, Some(0));
    state.select_next(3);
    state.select_next(3);
    state.select_next(3);
    assert_eq!(state.selected, Some(2));
    state.select_previous(3);
    assert_eq!(state.selected, Some(1));
    let mut state = WidgetListState::new();
    state.select_previous(3);
    assert_eq!(state.selected, Some(2));
    state.selected = Some(0);
    state.select_previous(3);
    assert_eq!(state.selected, Some(0));
    state.select_next(0);
    assert_eq!(state.selected, None);
}

#[test]
fn measure_sums_item_heights_up_to_the_available_rows() {
    let widget = list(WidgetListState::new());
    assert_eq!(
        widget.measure(lunatui_core::Size::new(6, 40)),
        lunatui_core::Size::new(6, 11)
    );
    assert_eq!(
        widget.measure(lunatui_core::Size::new(6, 5)),
        lunatui_core::Size::new(6, 5)
    );
    assert_eq!(
        widget.measure(lunatui_core::Size::new(0, 5)),
        lunatui_core::Size::new(0, 0)
    );
}

#[test]
fn padding_never_moves_the_selected_item_past_the_middle_of_the_area() {
    let huge = list(state(Some(2), 0, 0)).scroll_padding(10);
    let capped = list(state(Some(2), 0, 0)).scroll_padding(2);
    let area = Rect::new(0, 0, 6, 6);
    assert_eq!(huge.scrolled(area), capped.scrolled(area));
}

#[test]
fn padding_above_reveals_the_previous_item_when_the_selection_is_the_first_visible_item() {
    let scrolled = list(state(Some(3), 3, 0))
        .scroll_padding(2)
        .scrolled(Rect::new(0, 0, 6, 8));
    assert_eq!((scrolled.offset, scrolled.skip), (2, 0));
}

#[test]
fn a_selection_and_offset_past_the_end_clamp_to_the_last_item() {
    let scrolled = list(state(Some(9), 9, 0)).scrolled(Rect::new(0, 0, 6, 6));
    assert_eq!(scrolled.selected, Some(4));
    assert_eq!(scrolled.offset, 4);
}

#[test]
fn a_zero_width_or_zero_height_area_resets_the_state() {
    let widget = list(state(Some(3), 2, 1));
    for area in [Rect::new(0, 0, 0, 5), Rect::new(0, 0, 5, 0)] {
        assert_eq!(widget.scrolled(area), WidgetListState::new());
        assert_eq!(widget.item_areas(area), Vec::new());
    }
}

#[test]
fn stepping_back_from_a_selection_past_the_end_lands_on_the_last_item() {
    let mut state = WidgetListState {
        selected: Some(7),
        offset: 0,
        skip: 0,
    };
    state.select_previous(3);
    assert_eq!(state.selected, Some(2));
}

#[test]
fn padding_above_takes_part_of_an_item_and_never_more_than_asked() {
    let scrolled = list(state(Some(3), 3, 0))
        .scroll_padding(3)
        .scrolled(Rect::new(0, 0, 6, 8));
    assert_eq!((scrolled.offset, scrolled.skip), (1, 2));
}

#[test]
fn padding_above_is_capped_at_half_the_area() {
    let scrolled = list(state(Some(3), 3, 0))
        .scroll_padding(10)
        .scrolled(Rect::new(0, 0, 6, 4));
    assert_eq!((scrolled.offset, scrolled.skip), (2, 0));
}

#[test]
fn padding_above_the_first_item_has_nothing_to_reveal() {
    let scrolled = list(state(Some(0), 0, 0))
        .scroll_padding(3)
        .scrolled(Rect::new(0, 0, 6, 8));
    assert_eq!((scrolled.offset, scrolled.skip), (0, 0));
}

#[test]
fn item_areas_leave_out_hidden_and_zero_height_items() {
    let widget = WidgetList::new(
        4,
        |index, _| [2, 0, 1, 1][index],
        |_, _, _, _: &mut RenderCx<'_>| {},
    )
    .state(state(None, 0, 2));
    let indexes: Vec<usize> = widget
        .item_areas(Rect::new(0, 0, 4, 4))
        .iter()
        .map(|item| item.index)
        .collect();
    assert_eq!(indexes, [2, 3]);
}

#[test]
fn items_cut_by_an_edge_are_drawn_at_their_full_height() {
    let widget = WidgetList::new(
        3,
        |_, _| 4,
        |_, area, _, cx: &mut RenderCx<'_>| {
            for row in 0..area.height {
                cx.put_str(
                    area.x,
                    area.y + row,
                    &format!("h{}", area.height),
                    Style::new(),
                );
            }
        },
    )
    .state(state(None, 0, 2));
    let (grid, clusters) = render(&widget, 4, 4);
    assert_eq!(text(&grid, &clusters), ["h4", "h4", "h4", "h4"]);
}

#[test]
fn padding_above_skips_zero_height_items_without_counting_them() {
    let widget = WidgetList::new(
        4,
        |index, _| [2, 0, 1, 1][index],
        |_, _, _, _: &mut RenderCx<'_>| {},
    )
    .state(state(Some(2), 2, 0))
    .scroll_padding(1);
    let scrolled = widget.scrolled(Rect::new(0, 0, 4, 4));
    assert_eq!((scrolled.offset, scrolled.skip), (0, 1));
}

#[test]
fn items_that_fit_are_drawn_at_their_real_position_in_the_area() {
    let seen = std::cell::RefCell::new(Vec::new());
    let widget = WidgetList::new(
        2,
        |_, _| 2,
        |index, area, _, _: &mut RenderCx<'_>| seen.borrow_mut().push((index, area)),
    );
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(10, 8);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    widget.render(Rect::new(3, 2, 5, 4), &mut cx);
    assert_eq!(
        *seen.borrow(),
        [(0, Rect::new(3, 2, 5, 2)), (1, Rect::new(3, 4, 5, 2))]
    );
}
