// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_core::{Color, Rect, Size};

use crate::{RenderCx, Widget};

/// Limits Tab focus to a modal scope while active (for runtime focus managers).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FocusTrap {
    active: bool,
}

impl FocusTrap {
    /// Trap enabled: only `scope` ids receive focus.
    #[must_use]
    pub fn enabled() -> Self {
        Self { active: true }
    }

    /// Trap disabled.
    #[must_use]
    pub fn disabled() -> Self {
        Self { active: false }
    }

    /// Whether focus should stay inside the modal scope.
    #[must_use]
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Filters `tab_order` to `scope` when the trap is active.
    #[must_use]
    pub fn filter_tab_order(&self, scope: &[u32], tab_order: &[u32]) -> Vec<u32> {
        if !self.active {
            return tab_order.to_vec();
        }
        tab_order
            .iter()
            .filter(|id| scope.contains(id))
            .copied()
            .collect()
    }
}

/// Dimmed full-area backdrop plus a centered modal child.
#[derive(Clone, Debug)]
pub struct Overlay<W> {
    content: W,
    modal: Rect,
    backdrop_alpha: f32,
    backdrop_tint: Color,
    trap: FocusTrap,
}

impl<W> Overlay<W> {
    /// Modal `content` drawn in `modal` (absolute coordinates).
    #[must_use]
    pub fn new(content: W, modal: Rect) -> Self {
        Self {
            content,
            modal,
            backdrop_alpha: 0.55,
            backdrop_tint: Color::Rgb(0, 0, 0),
            trap: FocusTrap::enabled(),
        }
    }

    /// Backdrop translucency in `[0.0, 1.0]`.
    #[must_use]
    pub fn backdrop(mut self, alpha: f32, tint: Color) -> Self {
        self.backdrop_alpha = alpha.clamp(0.0, 1.0);
        self.backdrop_tint = tint;
        self
    }

    /// Focus confinement policy for the modal.
    #[must_use]
    pub fn focus_trap(mut self, trap: FocusTrap) -> Self {
        self.trap = trap;
        self
    }

    /// Access focus trap state for runtime wiring.
    #[must_use]
    pub fn traps_focus(&self) -> bool {
        self.trap.is_active()
    }
}

impl<W: Widget> Widget for Overlay<W> {
    fn render(&self, area: Rect, cx: &mut RenderCx<'_>) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        cx.blend_backdrop(area, self.backdrop_alpha, self.backdrop_tint);
        let modal = intersect(self.modal, area);
        if modal.width > 0 && modal.height > 0 {
            self.content.render(modal, cx);
        }
    }

    fn measure(&self, avail: Size) -> Size {
        if avail.width == 0 || avail.height == 0 {
            return Size::new(0, 0);
        }
        avail
    }
}

fn intersect(a: Rect, b: Rect) -> Rect {
    let x1 = a.x.max(b.x);
    let y1 = a.y.max(b.y);
    let x2 = a.x.saturating_add(a.width).min(b.x.saturating_add(b.width));
    let y2 = a.y.saturating_add(a.height).min(b.y.saturating_add(b.height));
    if x2 <= x1 || y2 <= y1 {
        return Rect::new(0, 0, 0, 0);
    }
    Rect::new(x1, y1, x2 - x1, y2 - y1)
}

/// Linear RGB blend used by [`RenderCx::blend_backdrop`].
#[must_use]
pub fn blend_rgb(base: (u8, u8, u8), overlay: (u8, u8, u8), alpha: f32) -> (u8, u8, u8) {
    let a = alpha.clamp(0.0, 1.0);
    let blend = |base_byte: u8, over_byte: u8| -> u8 {
        let b = f32::from(base_byte);
        let o = f32::from(over_byte);
        u8::try_from((b * (1.0 - a) + o * a).round() as u32).unwrap_or(base_byte)
    };
    (
        blend(base.0, overlay.0),
        blend(base.1, overlay.1),
        blend(base.2, overlay.2),
    )
}
