// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;

/// Builds the truecolor gradient frame: `─` on every third diagonal, `x` elsewhere,
/// a per-cell RGB foreground, and a dark background on every tenth column.
#[must_use]
pub fn frame(area: Rect) -> Buffer {
    let mut buf = Buffer::empty(area);
    for y in 0..area.height {
        for x in 0..area.width {
            let cell = &mut buf[(x, y)];
            cell.set_symbol(if (x + y).is_multiple_of(3) {
                "─"
            } else {
                "x"
            });
            cell.fg = Color::Rgb(channel(x), channel(y * 3), 128);
            if x.is_multiple_of(10) {
                cell.bg = Color::Rgb(20, 20, 30);
            }
        }
    }
    buf
}

fn channel(value: u16) -> u8 {
    u8::try_from(value % 255).unwrap_or(u8::MAX)
}
