// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use std::cell::RefCell;

use lunatui_core::{ClusterTable, Grid, Rect, Size};
use lunatui_data_table::{
    Column, Constraint, DataTable, RowSource, SortDirection, TableState, VecSource,
};
use lunatui_theme::{Theme, Variant};
use lunatui_widgets::{RenderCx, RowBuilder, Widget};

fn draw(w: u16, h: u16, widget: &dyn Widget) -> (Grid, ClusterTable) {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(w, h);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    widget.render(Rect::new(0, 0, w, h), &mut cx);
    (grid, clusters)
}

fn columns() -> Vec<Column<'static>> {
    vec![
        Column::new("Name", Constraint::Length(8)),
        Column::new("State", Constraint::Fill(1)),
    ]
}

fn source(rows: &[(&str, &str)]) -> VecSource {
    let mut source = VecSource::new();
    for (name, state) in rows {
        source.push_row(vec![(*name).to_owned(), (*state).to_owned()]);
    }
    source
}

struct CountingSource {
    rows: Vec<Vec<String>>,
    read: RefCell<Vec<usize>>,
}

impl CountingSource {
    fn new(count: usize) -> Self {
        Self {
            rows: (0..count)
                .map(|i| vec![format!("row-{i}"), "ok".to_owned()])
                .collect(),
            read: RefCell::new(Vec::new()),
        }
    }

    fn read_rows(&self) -> Vec<usize> {
        let mut rows = self.read.borrow().clone();
        rows.sort_unstable();
        rows.dedup();
        rows
    }
}

impl RowSource for CountingSource {
    fn len(&self) -> usize {
        self.rows.len()
    }

    fn row(&self, index: usize, out: &mut RowBuilder) {
        self.read.borrow_mut().push(index);
        out.clear();
        if let Some(row) = self.rows.get(index) {
            out.cells.extend(row.iter().cloned());
        }
    }

    fn cell(&self, index: usize, column: usize) -> Option<&str> {
        self.read.borrow_mut().push(index);
        self.rows
            .get(index)
            .map(|row| row.get(column).map_or("", String::as_str))
    }
}

#[test]
fn draws_frame_header_and_rows() {
    let table = DataTable::new(
        source(&[("api", "live"), ("web", "live"), ("auth", "down")]),
        columns(),
    );
    let (grid, clusters) = draw(24, 8, &table);
    let text = grid.display_text(&clusters);
    let rows: Vec<&str> = text.lines().collect();
    assert!(rows[0].starts_with('\u{256d}'), "{rows:?}");
    assert!(
        rows[1].contains("Name") && rows[1].contains("State"),
        "{rows:?}"
    );
    assert!(
        rows[2].contains("api") && rows[2].contains("live"),
        "{rows:?}"
    );
    assert!(
        rows[4].contains("auth") && rows[4].contains("down"),
        "{rows:?}"
    );
    assert!(rows[7].starts_with('\u{2570}'), "{rows:?}");
}

#[test]
fn reads_only_the_visible_rows_at_the_offset() {
    let source = CountingSource::new(1000);
    let table = DataTable::new(&source, columns()).offset(5);
    let (grid, clusters) = draw(24, 6, &table);
    assert_eq!(source.read_rows(), vec![5, 6, 7]);
    let text = grid.display_text(&clusters);
    assert!(text.contains("row-5"), "{text}");
    assert!(!text.contains("row-4"), "{text}");
    assert!(!text.contains("row-8"), "{text}");
}

#[test]
fn scrollbar_shows_only_when_rows_overflow() {
    let many = CountingSource::new(50);
    let table = DataTable::new(&many, columns());
    let (grid, clusters) = draw(24, 6, &table);
    let text = grid.display_text(&clusters);
    assert!(text.contains('\u{2588}'), "no thumb in {text:?}");
    assert!(text.contains('\u{2502}'), "no track in {text:?}");

    let few = DataTable::new(source(&[("api", "live")]), columns());
    let (grid, clusters) = draw(24, 6, &few);
    let text = grid.display_text(&clusters);
    assert!(!text.contains('\u{2588}'), "thumb in {text:?}");

    let hidden = DataTable::new(CountingSource::new(50), columns()).scrollbar(false);
    let (grid, clusters) = draw(24, 6, &hidden);
    let text = grid.display_text(&clusters);
    assert!(!text.contains('\u{2588}'), "thumb in {text:?}");
}

#[test]
fn state_sort_reorders_the_rows_shown() {
    let source = source(&[("web", "live"), ("api", "live"), ("mail", "down")]);
    let mut state = TableState::new();
    state.sort(&source, 0, SortDirection::Ascending);
    let table = DataTable::new(&source, columns()).state(&state);
    let (grid, clusters) = draw(24, 8, &table);
    let text = grid.display_text(&clusters);
    let rows: Vec<&str> = text.lines().collect();
    assert!(rows[2].contains("api"), "{rows:?}");
    assert!(rows[3].contains("mail"), "{rows:?}");
    assert!(rows[4].contains("web"), "{rows:?}");
    assert!(rows[1].contains('\u{25b2}'), "no sort marker in {rows:?}");
}

#[test]
fn empty_source_shows_the_empty_text_under_the_header() {
    let table = DataTable::new(VecSource::new(), columns());
    let (grid, clusters) = draw(24, 6, &table);
    let text = grid.display_text(&clusters);
    let rows: Vec<&str> = text.lines().collect();
    assert!(rows[1].contains("Name"), "{rows:?}");
    assert!(rows[2].contains("No data"), "{rows:?}");

    let quiet = DataTable::new(VecSource::new(), columns()).empty_text("");
    let (grid, clusters) = draw(24, 6, &quiet);
    assert!(!grid.display_text(&clusters).contains("No data"));
}

#[test]
fn sync_reports_fitting_rows_and_clamps_the_cursor() {
    let three = DataTable::new(source(&[("a", "x"), ("b", "y"), ("c", "z")]), columns());
    let area = Rect::new(0, 0, 24, 8);
    assert_eq!(three.data_rows(area), 5);
    assert_eq!(three.data_rows(Rect::new(0, 0, 24, 3)), 0);
    let borderless = DataTable::new(source(&[("a", "x")]), columns()).bordered(false);
    assert_eq!(borderless.data_rows(area), 7);

    let mut state = TableState::new();
    three.sync(area, &mut state);
    state.set_cursor(Some(2));
    assert_eq!(state.cursor(), Some(2));
    let one = DataTable::new(source(&[("a", "x")]), columns());
    one.sync(area, &mut state);
    assert_eq!(state.cursor(), Some(0));
}

#[test]
fn tiny_areas_draw_nothing_outside_themselves() {
    let table = DataTable::new(source(&[("api", "live")]), columns());
    for (w, h) in [(0, 0), (1, 1), (2, 2), (4, 3), (3, 8)] {
        let (grid, clusters) = draw(w, h, &table);
        let text = grid.display_text(&clusters);
        assert!(
            text.lines()
                .all(|row| row.chars().count() <= usize::from(w)),
            "{w}x{h}: {text:?}"
        );
    }
}

#[test]
fn measure_counts_frame_header_and_rows_within_avail() {
    let three = DataTable::new(source(&[("a", "x"), ("b", "y"), ("c", "z")]), columns());
    assert_eq!(three.measure(Size::new(40, 24)), Size::new(40, 6));
    assert_eq!(three.measure(Size::new(40, 4)), Size::new(40, 4));
    assert_eq!(three.measure(Size::new(0, 0)), Size::new(0, 0));
    let empty = DataTable::new(VecSource::new(), columns());
    assert_eq!(empty.measure(Size::new(40, 24)), Size::new(40, 4));
    let borderless = DataTable::new(source(&[("a", "x")]), columns()).bordered(false);
    assert_eq!(borderless.measure(Size::new(40, 24)), Size::new(40, 2));
}
