// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Rect};
use ratatui::text::Span;
use ratatui::widgets::{Row, Table, Widget};

const COLUMNS: [Constraint; 4] = [
    Constraint::Length(12),
    Constraint::Length(6),
    Constraint::Length(7),
    Constraint::Fill(1),
];

/// Builds `count` check rows: name, state, time and file.
#[must_use]
pub fn rows(count: usize) -> Vec<[String; 4]> {
    (0..count)
        .map(|i| {
            [
                format!("check-{i:04}"),
                "pass".to_owned(),
                format!("{i}ms"),
                format!("src/module/file_{i}.rs"),
            ]
        })
        .collect()
}

/// Builds a table from the first `built` of `rows` and renders it into `area` of `buf`.
pub fn render(rows: &[[String; 4]], built: usize, area: Rect, buf: &mut Buffer) {
    let rows = rows
        .iter()
        .take(built)
        .map(|row| Row::new(row.iter().map(|cell| Span::raw(cell.as_str()))));
    Table::new(rows, COLUMNS).render(area, buf);
}
