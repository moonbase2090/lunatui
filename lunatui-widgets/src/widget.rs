// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_core::Rect;

use crate::{RenderCx, Size};

/// Drawable lunatui component (PRD §3.2).
///
/// Widgets borrow their data, take `&self` in [`render`](Widget::render), and keep
/// cross-frame state in the app keyed by widget ids (DD-3).
pub trait Widget {
    /// Paints into `area` using `cx` (grid, clusters, theme).
    fn render(&self, area: Rect, cx: &mut RenderCx);

    /// Intrinsic size before layout clamping.
    fn measure(&self, avail: Size) -> Size;
}
