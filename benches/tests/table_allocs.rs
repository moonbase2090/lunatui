// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use std::hint::black_box;

use lunatui_bench::CountingAllocator;
use lunatui_core::{ClusterTable, Color, Grid, Rect, Style};
use lunatui_layout::Constraint;
use lunatui_theme::{Theme, Variant};
use lunatui_widgets::{RenderCx, RowSource, SortDirection, Table, TableState, VecSource, Widget};

#[global_allocator]
static ALLOC: CountingAllocator = CountingAllocator::system();

#[test]
fn drawing_a_sorted_selected_resized_table_allocates_nothing() {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(60, 14);
    let mut clusters = ClusterTable::new();
    let screen = Rect::new(0, 0, 60, 14);
    let source = VecSource::from_rows(
        (0..500)
            .map(|n| {
                vec![
                    format!("service-{n:03}"),
                    format!("{}", (n * 37) % 900),
                    String::from(if n % 3 == 0 { "eu" } else { "us" }),
                ]
            })
            .collect(),
    );
    let mut state = TableState::new();
    state.sort(&source, 1, SortDirection::Descending);
    state.set_column_width(0, Some(16));
    let columns = vec![
        Constraint::Length(12),
        Constraint::Length(8),
        Constraint::Fill(1),
    ];
    let rows = Table::new(state.view(&source), columns.clone())
        .header([
            ("Service", Style::new()),
            ("p95", Style::new()),
            ("Region", Style::new()),
        ])
        .data_rows(screen);
    state.fit(source.len(), rows);
    state.set_cursor(Some(40));
    state.down(3, true);
    let warn = |_: usize, column: usize| {
        if column == 1 {
            Style::new().fg(Color::Rgb(250, 180, 90))
        } else {
            Style::new()
        }
    };
    let table = Table::new(state.view(&source), columns)
        .header([
            ("Service", Style::new()),
            ("p95", Style::new()),
            ("Region", Style::new()),
        ])
        .state(&state)
        .highlight_style(Style::new().bg(Color::Rgb(60, 60, 120)))
        .selection_style(Style::new().bg(Color::Rgb(30, 60, 30)))
        .odd_row_style(Style::new().bg(Color::Rgb(20, 20, 30)))
        .highlight_symbol("▸", Style::new())
        .cell_styles(warn);
    table.render(screen, &mut RenderCx::new(&mut grid, &mut clusters, &theme));
    grid.fill(screen, Style::new(), ' ', &mut clusters);
    assert_eq!(grid.row_display(0, &clusters).trim_end(), "");
    let (columns_seen, counted) = ALLOC.measure(|| {
        let mut seen = 0_usize;
        for _ in 0..100 {
            let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
            black_box(&table).render(screen, &mut cx);
            seen += usize::from(black_box(&table).divider_at(screen, 16, 0).is_some());
        }
        seen
    });
    println!(
        "table renders=100; measured allocations={}",
        counted.allocations
    );
    assert_eq!(counted.allocations, 0);
    assert_eq!(columns_seen, 100);
    let header = grid.row_display(0, &clusters);
    assert!(header.starts_with(" Service         p95 ▼"), "{header}");
    let cursor = grid.row_display(14 - 1, &clusters);
    assert!(cursor.starts_with('▸'), "{cursor}");
}
