// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use std::borrow::Cow;

use lunatui_core::{Attrs, Color, Rect, Size, Style, clip_display_prefix, width};
use lunatui_theme::Token;

use crate::render_cx::clip_to_coordinates;
use crate::{RenderCx, Widget, WidgetState};

const DEFAULT_PADDING: u16 = 2;

/// How much a [`Button`] stands out from the buttons beside it.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ButtonVariant {
    /// A plain button on the raised surface.
    #[default]
    Normal,
    /// The main action, filled with the accent color.
    Primary,
    /// An action that destroys work, labelled in the fail color.
    Destructive,
}

/// A clickable label: a filled row of cells with its label centred inside.
///
/// Colors come from theme tokens, the [`ButtonVariant`], the [`WidgetState`] and whether it is
/// [`pressed`](Self::pressed):
///
/// - at rest, a normal or destructive button is `surface.1` with `text.primary` or `fail` text,
///   and a primary one is `accent` with `surface.0` text, bold;
/// - focused, it is filled with `focus.ring` (`fail` when destructive) with bold `surface.0` text;
/// - pressed, it sinks to `surface.0` with its color as bold text;
/// - disabled, it is `surface.1` with dimmed text whatever its variant.
///
/// The button fills the area it is given, with the label on the middle row, centred, and cut to
/// fit when the area is narrow. It measures as its label plus [`padding`](Self::padding) on each
/// side, one row high. [`hit_area`](Self::hit_area) gives the cells a click should reach, and
/// [`ButtonState`] turns presses and releases into clicks. A built button draws without
/// allocating.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Button<'a> {
    label: Cow<'a, str>,
    variant: ButtonVariant,
    state: WidgetState,
    pressed: bool,
    padding: u16,
    style: Option<Style>,
}

impl<'a> Button<'a> {
    /// A normal button showing `label`.
    #[must_use]
    pub fn new(label: impl Into<Cow<'a, str>>) -> Self {
        Self {
            label: label.into(),
            variant: ButtonVariant::Normal,
            state: WidgetState::Normal,
            pressed: false,
            padding: DEFAULT_PADDING,
            style: None,
        }
    }

    /// The variant, [`ButtonVariant::Normal`] by default.
    #[must_use]
    pub const fn variant(mut self, variant: ButtonVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Interaction state the button paints itself for.
    #[must_use]
    pub const fn state(mut self, state: WidgetState) -> Self {
        self.state = state;
        self
    }

    /// Draws the button held down, as while [`ButtonState::is_pressed`]. Ignored when disabled.
    #[must_use]
    pub const fn pressed(mut self, pressed: bool) -> Self {
        self.pressed = pressed;
        self
    }

    /// Blank cells on each side of the label when measured, two by default.
    #[must_use]
    pub const fn padding(mut self, cells: u16) -> Self {
        self.padding = cells;
        self
    }

    /// Draws the whole button in `style` instead of theme tokens.
    #[must_use]
    pub const fn style(mut self, style: Style) -> Self {
        self.style = Some(style);
        self
    }

    /// The label.
    #[must_use]
    pub fn label(&self) -> &str {
        &self.label
    }

    /// The cells a click should reach when the button is drawn in `area`: all of it, or none
    /// when the button is disabled.
    #[must_use]
    pub fn hit_area(&self, area: Rect) -> Rect {
        let area = clip_to_coordinates(area);
        if matches!(self.state, WidgetState::Disabled) || area.width == 0 || area.height == 0 {
            return Rect::new(area.x, area.y, 0, 0);
        }
        area
    }

    fn natural_width(&self) -> u16 {
        u16::try_from(width(&self.label))
            .unwrap_or(u16::MAX)
            .saturating_add(self.padding.saturating_mul(2))
    }

    fn colors(&self, cx: &RenderCx<'_>) -> Style {
        if let Some(style) = self.style {
            return style;
        }
        let surface: Color = cx.token(Token::Surface0);
        let raised: Color = cx.token(Token::Surface1);
        let text: Color = cx.token(Token::TextPrimary);
        let bold = Attrs::new().set_bold(true);
        if matches!(self.state, WidgetState::Disabled) {
            return Style::new()
                .fg(text)
                .bg(raised)
                .with_attrs(Attrs::new().set_dim(true));
        }
        let tone = match self.variant {
            ButtonVariant::Normal => cx.token(Token::FocusRing),
            ButtonVariant::Primary => cx.token(Token::Accent),
            ButtonVariant::Destructive => cx.token(Token::Fail),
        };
        if self.pressed {
            return Style::new().fg(tone).bg(surface).with_attrs(bold);
        }
        if matches!(self.state, WidgetState::Focused) {
            let fill = match self.variant {
                ButtonVariant::Destructive => tone,
                ButtonVariant::Normal | ButtonVariant::Primary => cx.token(Token::FocusRing),
            };
            return Style::new().fg(surface).bg(fill).with_attrs(bold);
        }
        match self.variant {
            ButtonVariant::Normal => Style::new().fg(text).bg(raised),
            ButtonVariant::Primary => Style::new().fg(surface).bg(tone).with_attrs(bold),
            ButtonVariant::Destructive => Style::new().fg(tone).bg(raised),
        }
    }
}

impl Widget for Button<'_> {
    fn render(&self, area: Rect, cx: &mut RenderCx<'_>) {
        let area = clip_to_coordinates(area);
        if area.width == 0 || area.height == 0 {
            return;
        }
        let style = self.colors(cx);
        cx.fill(area, style, ' ');
        cx.set_style(area, style);
        let label = clip_display_prefix(&self.label, area.width);
        let shown = u16::try_from(width(label)).unwrap_or(area.width);
        let x = area.x.saturating_add((area.width - shown) / 2);
        let y = area.y.saturating_add((area.height - 1) / 2);
        cx.put_str(x, y, label, style);
    }

    fn measure(&self, avail: Size) -> Size {
        Size::new(self.natural_width().min(avail.width), 1.min(avail.height))
    }
}

/// Whether a [`Button`] is held down, kept by the app between frames.
///
/// Keyboard: [`press`](Self::press) on Enter or Space, then [`release`](Self::release), which
/// reports the click. Pointer: [`pointer_down`](Self::pointer_down) on the button starts a press,
/// and [`pointer_up`](Self::pointer_up) clicks only when it ends on the same button, so a press
/// dragged away and released does not click. [`cancel`](Self::cancel) drops a press without a
/// click, as Escape does. Give the pointer methods [`Button::hit_area`], which is empty for a
/// disabled button, so a disabled button is never pressed by the pointer.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ButtonState {
    pressed: bool,
}

impl ButtonState {
    /// A button that is not held down.
    #[must_use]
    pub const fn new() -> Self {
        Self { pressed: false }
    }

    /// Whether the button is held down.
    #[must_use]
    pub const fn is_pressed(&self) -> bool {
        self.pressed
    }

    /// Holds the button down.
    pub fn press(&mut self) {
        self.pressed = true;
    }

    /// Lets the button go. Returns whether it was held down, which is a click.
    pub fn release(&mut self) -> bool {
        let clicked = self.pressed;
        self.pressed = false;
        clicked
    }

    /// Lets the button go without a click.
    pub fn cancel(&mut self) {
        self.pressed = false;
    }

    /// Holds the button down when `(column, row)` is in `hit`. Returns whether it is.
    pub fn pointer_down(&mut self, column: u16, row: u16, hit: Rect) -> bool {
        self.pressed = contains(hit, column, row);
        self.pressed
    }

    /// Lets the button go, and returns whether that was a click: it was held down and the
    /// pointer is still in `hit`.
    pub fn pointer_up(&mut self, column: u16, row: u16, hit: Rect) -> bool {
        self.release() && contains(hit, column, row)
    }
}

fn contains(area: Rect, column: u16, row: u16) -> bool {
    column >= area.x && column - area.x < area.width && row >= area.y && row - area.y < area.height
}
