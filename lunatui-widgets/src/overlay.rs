// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_core::{Color, Rect, Size, Style};

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
///
/// The modal area is cleared before the content is drawn, so nothing from the
/// backdrop shows through it.
#[derive(Clone, Debug)]
pub struct Overlay<W> {
    content: W,
    modal: Rect,
    backdrop_alpha: f32,
    backdrop_tint: Color,
    trap: FocusTrap,
    shadow: Option<Style>,
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
            shadow: None,
        }
    }

    /// Backdrop translucency in `[0.0, 1.0]`.
    #[must_use]
    pub fn backdrop(mut self, alpha: f32, tint: Color) -> Self {
        self.backdrop_alpha = alpha.clamp(0.0, 1.0);
        self.backdrop_tint = tint;
        self
    }

    /// Drop shadow: the modal's rectangle shifted one cell right and one down,
    /// filled with `style` behind the modal.
    #[must_use]
    pub fn shadow(mut self, style: Style) -> Self {
        self.shadow = Some(style);
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
        if let Some(shadow) = self.shadow {
            let offset = Rect::new(
                self.modal.x.saturating_add(1),
                self.modal.y.saturating_add(1),
                self.modal.width,
                self.modal.height,
            );
            fill_blank(cx, intersect(offset, area), shadow);
        }
        let modal = intersect(self.modal, area);
        if modal.width > 0 && modal.height > 0 {
            fill_blank(cx, modal, Style::new());
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

fn fill_blank(cx: &mut RenderCx<'_>, area: Rect, style: Style) {
    let blank = " ".repeat(usize::from(area.width));
    for y in area.y..area.y.saturating_add(area.height) {
        cx.put_str(area.x, y, &blank, style);
    }
}

fn intersect(a: Rect, b: Rect) -> Rect {
    let x1 = a.x.max(b.x);
    let y1 = a.y.max(b.y);
    let x2 = a.x.saturating_add(a.width).min(b.x.saturating_add(b.width));
    let y2 =
        a.y.saturating_add(a.height)
            .min(b.y.saturating_add(b.height));
    if x2 <= x1 || y2 <= y1 {
        return Rect::new(0, 0, 0, 0);
    }
    Rect::new(x1, y1, x2 - x1, y2 - y1)
}

/// Linear RGB blend used by [`RenderCx::blend_backdrop`].
#[must_use]
pub fn blend_rgb(base: (u8, u8, u8), overlay: (u8, u8, u8), alpha: f32) -> (u8, u8, u8) {
    const DENOM: u16 = 1024;
    let weight = overlay_weight(alpha.clamp(0.0, 1.0), DENOM);
    let blend = |base_byte: u8, over_byte: u8| -> u8 {
        let mixed = u32::from(base_byte) * u32::from(DENOM - weight)
            + u32::from(over_byte) * u32::from(weight);
        u8::try_from((mixed + u32::from(DENOM) / 2) / u32::from(DENOM)).unwrap_or(base_byte)
    };
    (
        blend(base.0, overlay.0),
        blend(base.1, overlay.1),
        blend(base.2, overlay.2),
    )
}

fn overlay_weight(alpha: f32, denom: u16) -> u16 {
    if alpha <= 0.0 {
        return 0;
    }
    if alpha >= 1.0 {
        return denom;
    }
    let mut lo = 0_u16;
    let mut hi = denom;
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        let frac = f32::from(mid) / f32::from(denom);
        if frac < alpha {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    lo.min(denom)
}
