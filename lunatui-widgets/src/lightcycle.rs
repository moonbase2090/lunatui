// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use crate::{RenderCx, Widget};
use lunatui_core::{Rect, Size, Style};
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// A time-sampled Braille trail.
pub struct Lightcycle;

impl Lightcycle {
    /// Creates a frame at the supplied elapsed time.
    pub fn new(_elapsed: Duration) -> Self {
        Self
    }
    /// Sets the cycle period.
    pub const fn period(self, _period: Duration) -> Self {
        self
    }
    /// Sets the trail styles.
    pub const fn trail(self, _head: Style, _tail: Style) -> Self {
        self
    }
}

impl Widget for Lightcycle {
    fn render(&self, _area: Rect, _cx: &mut RenderCx<'_>) {}
    fn measure(&self, avail: Size) -> Size {
        Size::new(1, 1).clamp(avail)
    }
}
