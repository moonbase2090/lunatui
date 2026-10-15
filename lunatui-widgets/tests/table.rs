// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::{ClusterTable, Grid, Rect};
use lunatui_layout::Constraint;
use lunatui_theme::{Theme, Variant};
use lunatui_widgets::{RenderCx, Table, VecSource, Widget};

#[test]
fn table_renders_visible_rows_only() {
    let mut source = VecSource::new();
    for i in 0..1000 {
        source.push_row(vec![format!("row-{i}"), "ok".to_owned()]);
    }
    let table = Table::new(source, vec![Constraint::Length(8), Constraint::Fill(1)])
        .offset(5)
        .selected(Some(6));
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(20, 3);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    table.render(Rect::new(0, 0, 20, 3), &mut cx);
    assert!(grid.row_display(0, &clusters).contains("row-5"));
    assert!(grid.row_display(1, &clusters).contains("row-6"));
}
