// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_core::{ClusterTable, Grid, Rect};
use lunatui_text::wrap::Wrap;
use lunatui_theme::{Theme, Variant};
use lunatui_widgets::{RenderCx, TextView};

/// Builds a log of `lines` numbered lint warnings, one per line.
#[must_use]
pub fn text(lines: usize) -> String {
    let mut text = (0..lines)
        .map(|i| {
            format!(
                "[{i:04}] lint warning in src/lib.rs line {i} consider removing the borrow here"
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    if lines > 0 {
        text.push('\n');
    }
    text
}

/// Renders `log` scrolled down by `scroll` visual rows into a `width`×`height` grid.
pub fn render(log: &str, scroll: u16, width: u16, height: u16) {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(width, height);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    let mut view = TextView::new(log)
        .wrap(Wrap { trim: true })
        .scroll((scroll, 0));
    view.render(Rect::new(0, 0, width, height), &mut cx);
}
