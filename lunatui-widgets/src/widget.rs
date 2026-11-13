// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_core::{Rect, Size};

use crate::RenderCx;

/// Drawable lunatui component (PRD §3.2).
///
/// Widgets borrow their data, take `&self` in [`render`](Widget::render), and keep
/// cross-frame state in the app keyed by widget ids (DD-3).
///
/// # Example
///
/// ```
/// use lunatui_core::{ClusterTable, Grid, Rect, Size, Style};
/// use lunatui_theme::{Theme, Token, Variant};
/// use lunatui_widgets::{RenderCx, Widget};
///
/// struct StatusDot;
///
/// impl Widget for StatusDot {
///     fn render(&self, area: Rect, cx: &mut RenderCx<'_>) {
///         if area.width == 0 || area.height == 0 {
///             return;
///         }
///         let color = cx.token(Token::Ok);
///         cx.put_str(area.x, area.y, "●", Style::new().fg(color));
///     }
///
///     fn measure(&self, avail: Size) -> Size {
///         Size::new(1, 1).clamp(avail)
///     }
/// }
///
/// let theme = Theme::builtin(Variant::Dark);
/// let mut grid = Grid::new(3, 1);
/// let mut clusters = ClusterTable::new();
/// let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
/// StatusDot.render(Rect::new(0, 0, 1, 1), &mut cx);
/// ```
pub trait Widget {
    /// Paints into `area` using `cx` (grid, clusters, theme).
    fn render(&self, area: Rect, cx: &mut RenderCx<'_>);

    /// Intrinsic size before layout clamping.
    fn measure(&self, avail: Size) -> Size;

    /// A visible control label that can supply an accessible name. Input
    /// widgets must not return their editable value here.
    fn control_name(&self) -> Option<&str> {
        None
    }

    /// Role of a control body when used in a labeled Field.
    fn control_role(&self) -> Option<lunatui_runtime::AccessibilityRole> {
        None
    }

    /// Whether a named wrapper should reject activation of this widget.
    fn control_disabled(&self) -> bool {
        false
    }
}
