// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::{Attrs, Cell, ClusterTable, Color, Grid, Rect, Style};
use lunatui_layout::Constraint;
use lunatui_theme::{Theme, Variant};
use lunatui_widgets::{
    Alignment, RenderCx, RowBuilder, RowSource, SortDirection, Table, TableState, VecSource, Widget,
};

const PICK: Color = Color::Rgb(40, 40, 90);
const RANGE: Color = Color::Rgb(20, 60, 20);

fn services() -> VecSource {
    VecSource::from_rows(
        [
            ["web", "9", "eu"],
            ["api", "120", "us"],
            ["Auth", "15", "eu"],
            ["cache", "9", "ap"],
            ["billing", "1,200", "us"],
        ]
        .iter()
        .map(|row| row.iter().map(|cell| (*cell).to_owned()).collect())
        .collect(),
    )
}

fn columns() -> Vec<Constraint> {
    vec![
        Constraint::Length(8),
        Constraint::Length(6),
        Constraint::Fill(1),
    ]
}

fn draw(table: &impl Widget, width: u16, height: u16) -> Grid {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(width, height);
    let mut clusters = ClusterTable::new();
    table.render(
        Rect::new(0, 0, width, height),
        &mut RenderCx::new(&mut grid, &mut clusters, &theme),
    );
    grid
}

fn rows(table: &impl Widget, width: u16, height: u16) -> Vec<String> {
    let grid = draw(table, width, height);
    let clusters = ClusterTable::new();
    (0..height)
        .map(|y| grid.row_display(y, &clusters))
        .collect()
}

fn style_at(grid: &Grid, x: u16, y: u16) -> Style {
    grid.get(x, y).map(Cell::style).unwrap_or_default()
}

fn header() -> [(&'static str, Style); 3] {
    [
        ("Service", Style::new()),
        ("p95", Style::new()),
        ("Region", Style::new()),
    ]
}

fn sorted(column: usize, direction: SortDirection) -> TableState {
    let mut state = TableState::new();
    state.sort(&services(), column, direction);
    state
}

#[test]
fn sorting_orders_numbers_by_value_and_text_without_case() {
    let source = services();
    let state = sorted(1, SortDirection::Ascending);
    let table = Table::new(state.view(&source), columns())
        .header(header())
        .state(&state);
    assert_eq!(
        rows(&table, 20, 6),
        [
            "Service p95 ▲ Region",
            "web     9     eu    ",
            "cache   9     ap    ",
            "Auth    15    eu    ",
            "api     120   us    ",
            "billing 1,200 us    ",
        ]
    );
    let by_name = sorted(0, SortDirection::Descending);
    let names: Vec<usize> = (0..5).map(|row| by_name.source_row(row)).collect();
    assert_eq!(names, [0, 3, 4, 2, 1]);
    assert_eq!(by_name.sorted(), Some((0, SortDirection::Descending)));
}

#[test]
fn equal_keys_keep_source_order_in_both_directions() {
    let ascending = sorted(2, SortDirection::Ascending);
    let order: Vec<usize> = (0..5).map(|row| ascending.source_row(row)).collect();
    assert_eq!(order, [3, 0, 2, 1, 4]);
    let descending = sorted(2, SortDirection::Descending);
    let order: Vec<usize> = (0..5).map(|row| descending.source_row(row)).collect();
    assert_eq!(order, [1, 4, 0, 2, 3]);
}

#[test]
fn sorting_keeps_the_cursor_on_its_source_row() {
    let source = services();
    let mut state = TableState::new();
    Table::new(&source, columns()).sync(Rect::new(0, 0, 20, 6), &mut state);
    state.set_cursor(Some(2));
    state.down(1, true);
    state.sort(&source, 1, SortDirection::Ascending);
    assert_eq!(state.cursor(), Some(1));
    assert_eq!(state.source_row(1), 3);
    assert_eq!(state.selection(), Some(1..=1));
    state.toggle_sort(&source, 1);
    assert_eq!(state.sorted(), Some((1, SortDirection::Descending)));
    assert_eq!(state.source_row(state.cursor().unwrap_or(0)), 3);
    state.toggle_sort(&source, 0);
    assert_eq!(state.sorted(), Some((0, SortDirection::Ascending)));
    state.clear_sort();
    assert_eq!(state.sorted(), None);
    assert_eq!(state.cursor(), Some(3));
    assert_eq!(state.source_row(4), 4);
}

#[test]
fn the_sort_marker_follows_alignment_and_narrow_columns() {
    let source = services();
    let right = Table::new(&source, columns())
        .header(header())
        .alignments(vec![Alignment::Left, Alignment::Right])
        .sort_indicator(Some((1, SortDirection::Descending)));
    assert_eq!(rows(&right, 20, 1)[0], "Service  p95 ▼Region");
    let narrow = Table::new(&source, vec![Constraint::Length(1), Constraint::Length(3)])
        .header([("Service", Style::new()), ("p95", Style::new())])
        .sort_indicator(Some((0, SortDirection::Ascending)));
    assert_eq!(rows(&narrow, 4, 1)[0], "▲p95");
}

#[test]
fn a_range_selection_is_drawn_apart_from_the_cursor_row() {
    let source = services();
    let mut state = TableState::new();
    let table = Table::new(&source, columns());
    table.sync(Rect::new(0, 0, 20, 5), &mut state);
    state.set_cursor(Some(1));
    state.down(2, true);
    assert_eq!(state.selection(), Some(1..=3));
    let table = Table::new(&source, columns())
        .state(&state)
        .highlight_style(Style::new().bg(PICK))
        .selection_style(Style::new().bg(RANGE));
    let grid = draw(&table, 20, 5);
    assert_eq!(style_at(&grid, 0, 0).bg, Color::Default);
    assert_eq!(style_at(&grid, 0, 1).bg, RANGE);
    assert_eq!(style_at(&grid, 19, 2).bg, RANGE);
    assert_eq!(style_at(&grid, 0, 3).bg, PICK);
    assert_eq!(style_at(&grid, 0, 4).bg, Color::Default);
    let selected: Vec<usize> = state.selected_source_rows().collect();
    assert_eq!(selected, [1, 2, 3]);
    state.up(3, false);
    assert_eq!(state.selection(), Some(0..=0));
}

#[test]
fn the_cursor_moves_pages_and_selects_all() {
    let source = VecSource::from_rows((0..20).map(|n| vec![n.to_string()]).collect());
    let mut state = TableState::new();
    let table = Table::new(&source, vec![Constraint::Fill(1)]).header([("n", Style::new())]);
    table.sync(Rect::new(0, 0, 10, 6), &mut state);
    assert_eq!(state.cursor(), None);
    state.down(1, false);
    assert_eq!(state.cursor(), Some(0));
    state.page_down(false);
    assert_eq!((state.cursor(), state.offset()), (Some(5), 1));
    state.page_down(true);
    assert_eq!(state.selection(), Some(5..=10));
    assert_eq!(state.offset(), 6);
    state.page_up(false);
    assert_eq!((state.cursor(), state.offset()), (Some(5), 5));
    state.down(100, false);
    assert_eq!((state.cursor(), state.offset()), (Some(19), 15));
    state.select_all();
    assert_eq!(state.selection(), Some(0..=19));
    state.clear_selection();
    assert_eq!(state.selection(), Some(19..=19));
    state.up(100, false);
    assert_eq!((state.cursor(), state.offset()), (Some(0), 0));
    assert!(state.is_selected(0));
    assert!(!state.is_selected(1));
    let gapped = Table::new(&source, vec![Constraint::Fill(1)]).row_gap(1);
    gapped.sync(Rect::new(0, 0, 10, 5), &mut state);
    state.down(3, false);
    assert_eq!(state.offset(), 1);
}

#[test]
fn sync_drops_a_stale_sort_and_clamps_the_cursor() {
    let mut state = sorted(1, SortDirection::Ascending);
    let fewer = VecSource::from_rows(vec![vec!["a".to_owned()], vec!["b".to_owned()]]);
    state.set_cursor(Some(4));
    Table::new(&fewer, columns()).sync(Rect::new(0, 0, 20, 4), &mut state);
    assert_eq!(state.sorted(), None);
    assert_eq!(state.cursor(), Some(1));
    assert_eq!(state.source_row(1), 1);
    let empty = VecSource::new();
    Table::new(&empty, columns()).sync(Rect::new(0, 0, 20, 4), &mut state);
    assert_eq!(state.cursor(), None);
    state.down(1, false);
    assert_eq!(state.cursor(), None);
    state.select_all();
    assert_eq!(state.selection(), None);
}

#[test]
fn column_widths_from_the_state_replace_constraints() {
    let source = services();
    let mut state = TableState::new();
    state.set_column_width(1, Some(3));
    let table = Table::new(&source, columns())
        .header(header())
        .state(&state);
    let area = Rect::new(0, 0, 20, 3);
    assert_eq!(
        table.column_areas(area),
        [
            Rect::new(0, 0, 8, 3),
            Rect::new(8, 0, 3, 3),
            Rect::new(11, 0, 9, 3)
        ]
    );
    assert_eq!(rows(&table, 20, 2)[1], "web     9  eu       ");
    assert_eq!(state.column_width(1), Some(3));
    assert_eq!(state.column_width(4), None);
    state.set_column_width(1, Some(0));
    assert_eq!(state.column_width(1), Some(1));
    state.set_column_width(1, None);
    assert_eq!(state.column_width(1), None);
}

#[test]
fn dragging_a_header_divider_resizes_its_column() {
    let source = services();
    let table = Table::new(&source, columns()).header(header());
    let area = Rect::new(0, 0, 20, 4);
    assert_eq!(table.divider_at(area, 7, 0), Some(0));
    assert_eq!(table.divider_at(area, 8, 0), Some(0));
    assert_eq!(table.divider_at(area, 6, 0), None);
    assert_eq!(table.divider_at(area, 13, 0), Some(1));
    assert_eq!(table.divider_at(area, 7, 1), None);
    assert_eq!(Table::new(&source, columns()).divider_at(area, 7, 0), None);
    assert_eq!(table.column_at(area, 9, 2), Some(1));
    assert_eq!(table.column_at(area, 9, 4), None);

    let mut state = TableState::new();
    state.start_resize(0, 8, 8);
    assert_eq!(state.resizing(), Some(0));
    state.drag_resize(12);
    assert_eq!(state.column_width(0), Some(12));
    state.drag_resize(2);
    assert_eq!(state.column_width(0), Some(2));
    state.drag_resize(0);
    assert_eq!(state.column_width(0), Some(1));
    state.end_resize();
    assert_eq!(state.resizing(), None);
    state.drag_resize(20);
    assert_eq!(state.column_width(0), Some(1));
}

#[test]
fn a_closure_styles_single_cells() {
    let source = services();
    let warn = |row: usize, column: usize| {
        if column == 1 && row == 1 {
            Style::new()
                .fg(Color::Rgb(250, 100, 100))
                .with_attrs(Attrs::new().set_bold(true))
        } else {
            Style::new()
        }
    };
    let table = Table::new(&source, columns()).cell_styles(warn);
    let grid = draw(&table, 20, 3);
    assert_eq!(style_at(&grid, 8, 1).fg, Color::Rgb(250, 100, 100));
    assert!(style_at(&grid, 8, 1).attrs.bold());
    assert_eq!(style_at(&grid, 8, 0).fg, Color::Default);
    assert_eq!(style_at(&grid, 0, 1).fg, Color::Default);
}

#[test]
fn blank_cells_carry_the_row_attributes() {
    let source = services();
    let table = Table::new(&source, columns())
        .selected(Some(0))
        .highlight_style(
            Style::new()
                .bg(PICK)
                .with_attrs(Attrs::new().set_bold(true)),
        );
    let grid = draw(&table, 20, 2);
    assert!(style_at(&grid, 19, 0).attrs.bold());
    assert!(!style_at(&grid, 19, 1).attrs.bold());
}

struct Copied(Vec<Vec<String>>);

impl RowSource for Copied {
    fn len(&self) -> usize {
        self.0.len()
    }

    fn row(&self, index: usize, out: &mut RowBuilder) {
        out.clear();
        out.cells.extend(self.0[index].iter().cloned());
    }
}

#[test]
fn a_source_without_borrowed_cells_still_renders_and_sorts() {
    let source = Copied(vec![
        vec!["b".to_owned(), "2".to_owned()],
        vec!["a".to_owned(), "1".to_owned()],
    ]);
    let mut state = TableState::new();
    state.sort(&source, 0, SortDirection::Ascending);
    let table = Table::new(
        state.view(&source),
        vec![Constraint::Length(2), Constraint::Length(2)],
    );
    assert_eq!(rows(&table, 4, 2), ["a 1 ", "b 2 "]);
}

#[test]
fn view_rows_read_through_the_order() {
    let source = services();
    let state = sorted(0, SortDirection::Ascending);
    let view = state.view(&source);
    assert_eq!(view.len(), 5);
    assert_eq!(view.cell(0, 0), Some("api"));
    assert_eq!(view.cell(4, 0), Some("web"));
    assert_eq!(view.cell(0, 7), Some(""));
    assert_eq!(view.cell(9, 0), None);
    let mut row = RowBuilder::default();
    view.row(1, &mut row);
    assert_eq!(row.cells, ["Auth", "15", "eu"]);
}

#[test]
fn data_rows_count_the_header_and_gaps() {
    let source = services();
    let plain = Table::new(&source, columns());
    assert_eq!(plain.data_rows(Rect::new(0, 0, 20, 6)), 6);
    let headed = Table::new(&source, columns()).header(header());
    assert_eq!(headed.data_rows(Rect::new(0, 0, 20, 6)), 5);
    let gapped = Table::new(&source, columns()).header(header()).row_gap(1);
    assert_eq!(gapped.data_rows(Rect::new(0, 0, 20, 6)), 3);
    assert_eq!(gapped.data_rows(Rect::new(0, 0, 20, 5)), 2);
    let mut state = TableState::new();
    let rows = Table::new(state.view(&source), columns()).data_rows(Rect::new(0, 0, 20, 2));
    state.fit(source.len(), rows);
    state.down(4, false);
    assert_eq!((state.cursor(), state.offset()), (Some(0), 0));
    state.down(4, false);
    assert_eq!((state.cursor(), state.offset()), (Some(4), 3));
}

#[test]
fn column_at_uses_the_area_rows_and_each_columns_own_cells() {
    let source = services();
    let table = Table::new(&source, columns());
    let area = Rect::new(0, 2, 20, 3);
    assert_eq!(table.column_at(area, 3, 1), None);
    assert_eq!(table.column_at(area, 3, 2), Some(0));
    assert_eq!(table.column_at(area, 3, 4), Some(0));
    assert_eq!(table.column_at(area, 3, 5), None);
    assert_eq!(table.column_at(area, 7, 3), Some(0));
    assert_eq!(table.column_at(area, 8, 3), Some(1));
    assert_eq!(table.column_at(area, 14, 3), Some(2));
}

#[test]
fn the_solved_columns_follow_the_area_and_the_gutter() {
    let source = services();
    let table = Table::new(&source, columns());
    assert_eq!(
        table.column_areas(Rect::new(0, 0, 20, 2))[2],
        Rect::new(14, 0, 6, 2)
    );
    assert_eq!(
        table.column_areas(Rect::new(0, 0, 30, 2))[2],
        Rect::new(14, 0, 16, 2)
    );
    let marked = Table::new(&source, columns()).highlight_symbol("> ", Style::new());
    assert_eq!(
        marked.column_areas(Rect::new(0, 0, 20, 2)),
        [
            Rect::new(2, 0, 8, 2),
            Rect::new(10, 0, 6, 2),
            Rect::new(16, 0, 4, 2)
        ]
    );
}

#[test]
fn the_sort_marker_fits_short_and_centred_columns() {
    let source = services();
    let two = Table::new(&source, vec![Constraint::Length(2), Constraint::Length(6)])
        .header([("Service", Style::new()), ("Errors", Style::new())])
        .sort_indicator(Some((0, SortDirection::Ascending)));
    assert_eq!(rows(&two, 8, 1)[0], " ▲Errors");
    let long = Table::new(&source, vec![Constraint::Length(8), Constraint::Length(6)])
        .header([("Service", Style::new()), ("Latency", Style::new())])
        .sort_indicator(Some((1, SortDirection::Descending)));
    assert_eq!(rows(&long, 14, 1)[0], "Service Late ▼");
    let centred = Table::new(&source, vec![Constraint::Length(9), Constraint::Length(9)])
        .header([("p95", Style::new()), ("Region", Style::new())])
        .alignments(vec![Alignment::Center, Alignment::Center])
        .sort_indicator(Some((0, SortDirection::Ascending)));
    assert_eq!(rows(&centred, 18, 1)[0], "  p95 ▲   Region  ");
    assert_eq!(rows(&centred, 18, 2)[1], "   web       9    ");
}

#[test]
fn rows_past_the_data_are_plain_and_rows_below_the_area_untouched() {
    let source = services();
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(20, 9);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    for y in 0..9 {
        cx.put_str(0, y, "....................", Style::new());
    }
    Table::new(&source, columns())
        .odd_row_style(Style::new().bg(RANGE))
        .render(Rect::new(0, 0, 20, 7), &mut cx);
    assert_eq!(style_at(&grid, 0, 5).bg, Color::Default);
    assert_eq!(style_at(&grid, 0, 6).bg, Color::Default);
    assert_eq!(style_at(&grid, 0, 3).bg, RANGE);
    assert_eq!(grid.row_display(6, &clusters), "                    ");
    assert_eq!(grid.row_display(7, &clusters), "....................");

    let mut short = Grid::new(20, 3);
    let mut cx = RenderCx::new(&mut short, &mut clusters, &theme);
    for y in 0..3 {
        cx.put_str(0, y, "....................", Style::new());
    }
    Table::new(&source, columns()).render(Rect::new(0, 0, 20, 2), &mut cx);
    assert_eq!(short.row_display(1, &clusters), "api     120   us    ");
    assert_eq!(short.row_display(2, &clusters), "....................");
}

#[test]
fn a_table_without_columns_draws_nothing() {
    let source = services();
    let table = Table::new(&source, Vec::new());
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(6, 2);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    cx.put_str(0, 0, "......", Style::new());
    table.render(Rect::new(0, 0, 6, 2), &mut cx);
    assert_eq!(grid.row_display(0, &clusters), "......");
    assert_eq!(
        Table::new(&source, columns()).measure(lunatui_core::Size::new(0, 5)),
        lunatui_core::Size::new(0, 0)
    );
    assert_eq!(
        Table::new(&source, columns()).measure(lunatui_core::Size::new(5, 0)),
        lunatui_core::Size::new(0, 0)
    );
    assert_eq!(
        Table::new(&source, columns()).measure(lunatui_core::Size::new(9, 9)),
        lunatui_core::Size::new(9, 5)
    );
}

#[test]
fn toggling_another_column_starts_ascending() {
    let source = services();
    let mut state = TableState::new();
    state.sort(&source, 1, SortDirection::Ascending);
    state.toggle_sort(&source, 0);
    assert_eq!(state.sorted(), Some((0, SortDirection::Ascending)));
}

#[test]
fn fit_clamps_the_anchor_into_fewer_rows() {
    let source = services();
    let mut state = TableState::new();
    Table::new(&source, columns()).sync(Rect::new(0, 0, 20, 5), &mut state);
    state.set_cursor(Some(3));
    state.down(1, true);
    assert_eq!(state.selection(), Some(3..=4));
    state.fit(3, 5);
    assert_eq!(state.selection(), Some(2..=2));
    state.fit(5, 5);
    state.set_cursor(Some(4));
    state.up(2, true);
    state.fit(2, 5);
    assert_eq!(state.selection(), Some(1..=1));
}

#[test]
fn numbers_with_units_sort_by_value() {
    let source = VecSource::from_rows(
        ["100 ms", "9 ms", "25 ms", "n/a"]
            .iter()
            .map(|cell| vec![(*cell).to_owned()])
            .collect(),
    );
    let mut state = TableState::new();
    state.sort(&source, 0, SortDirection::Ascending);
    let order: Vec<usize> = (0..4).map(|row| state.source_row(row)).collect();
    assert_eq!(order, [1, 2, 0, 3]);
}
