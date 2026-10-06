// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::{Paragraph, Widget, Wrap};

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

/// How the log paragraph lays out its lines.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Layout {
    /// Lines wrap at the area width, trimming leading whitespace.
    Wrapped,
    /// Lines are cut at the area width.
    Unwrapped,
}

/// Renders `log` scrolled down by `scroll` rows into `area` of `buf`.
pub fn render(log: &str, scroll: u16, layout: Layout, area: Rect, buf: &mut Buffer) {
    let paragraph = Paragraph::new(log).scroll((scroll, 0));
    match layout {
        Layout::Wrapped => paragraph.wrap(Wrap { trim: true }).render(area, buf),
        Layout::Unwrapped => paragraph.render(area, buf),
    }
}

/// Returns `count` lines of `log` starting at line `first`, as an app that
/// virtualizes its own scrolling would pass them.
#[must_use]
pub fn visible_slice(log: &str, first: usize, count: usize) -> String {
    log.lines()
        .skip(first)
        .take(count)
        .collect::<Vec<_>>()
        .join("\n")
}
