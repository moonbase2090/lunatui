// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_core::{ClusterTable, Grid, Rect};
use lunatui_layout::Constraint;
use lunatui_theme::{Theme, Variant};
use lunatui_widgets::{RenderCx, Table, VecSource, Widget};

const COLUMNS: [Constraint; 4] = [
    Constraint::Length(12),
    Constraint::Length(6),
    Constraint::Length(7),
    Constraint::Fill(1),
];

/// Builds `count` check rows: name, state, time and file.
#[must_use]
pub fn rows(count: usize) -> Vec<Vec<String>> {
    (0..count)
        .map(|i| {
            vec![
                format!("check-{i:04}"),
                "pass".to_owned(),
                format!("{i}ms"),
                format!("src/module/file_{i}.rs"),
            ]
        })
        .collect()
}

/// Renders the first `built` rows of `rows` into a `width`×`height` grid.
pub fn render(rows: &[Vec<String>], built: usize, width: u16, height: u16) {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(width, height);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    let slice: Vec<Vec<String>> = rows.iter().take(built).cloned().collect();
    let source = VecSource::from_rows(slice);
    let table = Table::new(source, COLUMNS.to_vec());
    table.render(Rect::new(0, 0, width, height), &mut cx);
}
