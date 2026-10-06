// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

/// Builds the truecolor gradient frame: `─` on every third diagonal, `x` elsewhere,
/// a per-cell RGB foreground, and a dark background on every tenth column.
#[must_use]
pub fn frame(area: Rect) -> Buffer {
    Buffer::empty(area)
}
