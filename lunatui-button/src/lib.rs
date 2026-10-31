// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::{Attrs, Rect, Size, Style, clip_display_prefix, width};
use lunatui_ui_tokens::{Role, Tone, surface};
use lunatui_widgets::{RenderCx, Widget};

pub use lunatui_widgets::{ButtonState, WidgetState};

/// How a [`Button`] stands out.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Variant {
    /// The main action, filled with the primary color.
    #[default]
    Default,
    /// A quiet raised fill.
    Secondary,
    /// Square brackets in the border color on the page background.
    Outline,
    /// Plain text that only fills when focused or pressed.
    Ghost,
    /// An action that destroys work, filled with the danger color.
    Destructive,
}

/// How much blank space a [`Button`] keeps on each side of its label.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ButtonSize {
    /// One cell.
    Small,
    /// Two cells.
    #[default]
    Medium,
    /// Four cells.
    Large,
}

impl ButtonSize {
    const fn padding(self) -> u16 {
        match self {
            Self::Small => 1,
            Self::Medium => 2,
            Self::Large => 4,
        }
    }
}

fn resting(cx: &RenderCx<'_>, variant: Variant) -> Style {
    match variant {
        Variant::Default => Tone::Primary.style(cx),
        Variant::Secondary => surface(cx, Role::Card),
        Variant::Outline | Variant::Ghost => surface(cx, Role::Background),
        Variant::Destructive => Tone::Danger.style(cx),
    }
}

fn paint(
    area: Rect,
    label: &str,
    brackets: bool,
    style: Style,
    edge: Style,
    cx: &mut RenderCx<'_>,
) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    cx.fill(area, style, ' ');
    cx.set_style(area, style);
    let text = clip_display_prefix(label, area.width);
    let shown = u16::try_from(width(text)).unwrap_or(area.width);
    let x = area.x + (area.width - shown) / 2;
    let y = area.y + (area.height - 1) / 2;
    cx.put_str(x, y, text, style);
    if brackets && area.width >= 2 {
        cx.put_str(area.x, y, "[", edge);
        cx.put_str(area.x + area.width - 1, y, "]", edge);
    }
}

fn state_style(
    cx: &RenderCx<'_>,
    base: Style,
    state: WidgetState,
    pressed: bool,
    tone_text: bool,
) -> Style {
    if matches!(state, WidgetState::Disabled) {
        return base.with_attrs(Attrs::new().set_dim(true));
    }
    let ring = cx.token(Role::Ring.token());
    let page = cx.token(Role::Background.token());
    let bold = Attrs::new().set_bold(true);
    if pressed {
        let fg = if tone_text { base.bg } else { ring };
        return Style::new().fg(fg).bg(page).with_attrs(bold);
    }
    if matches!(state, WidgetState::Focused) {
        return Style::new().fg(page).bg(ring).with_attrs(bold);
    }
    base
}

/// A clickable one-line label in a [`Variant`] and size.
///
/// Colors come from `lunatui-ui-tokens`: at rest the variant sets the look; focused it fills with
/// the ring color; pressed it sinks to the page background; disabled it dims. It fills the area
/// it is given with the label centred, and measures as the label plus padding. For clicks, keep a
/// [`ButtonState`] and pass it [`hit_area`](Self::hit_area).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Button<'a> {
    label: &'a str,
    variant: Variant,
    size: ButtonSize,
    state: WidgetState,
    pressed: bool,
}

impl<'a> Button<'a> {
    /// A default button showing `label`.
    #[must_use]
    pub const fn new(label: &'a str) -> Self {
        Self {
            label,
            variant: Variant::Default,
            size: ButtonSize::Medium,
            state: WidgetState::Normal,
            pressed: false,
        }
    }

    /// Sets the variant.
    #[must_use]
    pub const fn variant(mut self, variant: Variant) -> Self {
        self.variant = variant;
        self
    }

    /// Sets the size.
    #[must_use]
    pub const fn size(mut self, size: ButtonSize) -> Self {
        self.size = size;
        self
    }

    /// Sets the interaction state.
    #[must_use]
    pub const fn state(mut self, state: WidgetState) -> Self {
        self.state = state;
        self
    }

    /// Draws the button held down, as while [`ButtonState::is_pressed`].
    #[must_use]
    pub const fn pressed(mut self, pressed: bool) -> Self {
        self.pressed = pressed;
        self
    }

    /// The cells a click should reach in `area`: all of it, or none when disabled.
    #[must_use]
    pub const fn hit_area(&self, area: Rect) -> Rect {
        if matches!(self.state, WidgetState::Disabled) {
            return Rect::new(area.x, area.y, 0, 0);
        }
        area
    }
}

impl Widget for Button<'_> {
    fn render(&self, area: Rect, cx: &mut RenderCx<'_>) {
        let base = resting(cx, self.variant);
        let ghost = matches!(self.variant, Variant::Ghost | Variant::Outline);
        let style = state_style(cx, base, self.state, self.pressed, !ghost);
        let edge = if matches!(self.variant, Variant::Outline) && style == base {
            surface(cx, Role::Background).fg(cx.token(Role::Border.token()))
        } else {
            style
        };
        let brackets = matches!(self.variant, Variant::Outline);
        paint(area, self.label, brackets, style, edge, cx);
    }

    fn measure(&self, avail: Size) -> Size {
        let w = u16::try_from(width(self.label)).unwrap_or(u16::MAX);
        let w = w.saturating_add(self.size.padding().saturating_mul(2));
        Size::new(w, 1).clamp(avail)
    }
}

/// A button that stays pressed: filled with the primary color when on, outlined when off.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Toggle<'a> {
    label: &'a str,
    on: bool,
    size: ButtonSize,
    state: WidgetState,
}

impl<'a> Toggle<'a> {
    /// A toggle showing `label`, pressed when `on`.
    #[must_use]
    pub const fn new(label: &'a str, on: bool) -> Self {
        Self {
            label,
            on,
            size: ButtonSize::Medium,
            state: WidgetState::Normal,
        }
    }

    /// Sets the size.
    #[must_use]
    pub const fn size(mut self, size: ButtonSize) -> Self {
        self.size = size;
        self
    }

    /// Sets the interaction state.
    #[must_use]
    pub const fn state(mut self, state: WidgetState) -> Self {
        self.state = state;
        self
    }

    /// Whether the toggle is on.
    #[must_use]
    pub const fn is_on(&self) -> bool {
        self.on
    }

    /// The cells a click should reach in `area`: all of it, or none when disabled.
    #[must_use]
    pub const fn hit_area(&self, area: Rect) -> Rect {
        if matches!(self.state, WidgetState::Disabled) {
            return Rect::new(area.x, area.y, 0, 0);
        }
        area
    }
}

impl Widget for Toggle<'_> {
    fn render(&self, area: Rect, cx: &mut RenderCx<'_>) {
        let variant = if self.on {
            Variant::Default
        } else {
            Variant::Secondary
        };
        let base = resting(cx, variant);
        let style = state_style(cx, base, self.state, false, true);
        paint(area, self.label, false, style, style, cx);
    }

    fn measure(&self, avail: Size) -> Size {
        let w = u16::try_from(width(self.label)).unwrap_or(u16::MAX);
        Size::new(w.saturating_add(self.size.padding().saturating_mul(2)), 1).clamp(avail)
    }
}
