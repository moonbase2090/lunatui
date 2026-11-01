// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::{Attrs, Rect, Size, Style, clip_display_prefix, width};
use lunatui_ui_tokens::{Role, surface, text};
use lunatui_widgets::{RenderCx, Widget};

const MARKER: &str = " *";

/// A label, a control widget, and a help or error line under it.
#[derive(Clone, Copy)]
pub struct Field<'a> {
    label: &'a str,
    help: &'a str,
    error: &'a str,
    required: bool,
    focused: bool,
    disabled: bool,
    body: Option<&'a dyn Widget>,
    on: Role,
}

impl<'a> Field<'a> {
    /// A field with this label and no control, help or error.
    #[must_use]
    pub const fn new(label: &'a str) -> Self {
        Self {
            label,
            help: "",
            error: "",
            required: false,
            focused: false,
            disabled: false,
            body: None,
            on: Role::Background,
        }
    }

    /// Muted help text under the control, shown while there is no error.
    #[must_use]
    pub const fn help(mut self, help: &'a str) -> Self {
        self.help = help;
        self
    }

    /// A validation message under the control in the destructive color. It replaces the help.
    #[must_use]
    pub const fn error(mut self, error: &'a str) -> Self {
        self.error = error;
        self
    }

    /// Marks the label with a `*`.
    #[must_use]
    pub const fn required(mut self) -> Self {
        self.required = true;
        self
    }

    /// Draws the label in the ring color.
    #[must_use]
    pub const fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    /// Dims the label, the help and the error.
    #[must_use]
    pub const fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// The control drawn between the label and the message.
    #[must_use]
    pub const fn body(mut self, body: &'a dyn Widget) -> Self {
        self.body = Some(body);
        self
    }

    /// The surface role the field sits on, `Role::Background` by default.
    #[must_use]
    pub const fn on(mut self, on: Role) -> Self {
        self.on = on;
        self
    }

    fn message(&self) -> &'a str {
        if self.error.is_empty() {
            self.help
        } else {
            self.error
        }
    }

    fn dim(&self, style: Style) -> Style {
        if self.disabled {
            style.with_attrs(style.attrs.set_dim(true))
        } else {
            style
        }
    }

    fn body_rows(&self, avail: Size) -> u16 {
        self.body.map_or(0, |b| b.measure(avail).height.max(1))
    }
}

impl Widget for Field<'_> {
    fn render(&self, area: Rect, cx: &mut RenderCx<'_>) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let page = surface(cx, self.on);
        cx.fill(area, page, ' ');
        let label_color = if self.focused {
            cx.token(Role::Ring.token())
        } else {
            text(cx, Role::Foreground).fg
        };
        let label = self.dim(page.fg(label_color).with_attrs(Attrs::new().set_bold(true)));
        let shown = clip_display_prefix(self.label, area.width);
        cx.put_str(area.x, area.y, shown, label);
        if self.required {
            let used = u16::try_from(width(shown)).unwrap_or(area.width);
            let marker = self.dim(page.fg(text(cx, Role::Destructive).fg));
            let room = area.width.saturating_sub(used);
            cx.put_str(
                area.x.saturating_add(used),
                area.y,
                clip_display_prefix(MARKER, room),
                marker,
            );
        }
        let rest = area.height - 1;
        let body_rows = self.body_rows(Size::new(area.width, rest)).min(rest);
        if let Some(body) = self.body
            && body_rows > 0
        {
            body.render(
                Rect::new(area.x, area.y.saturating_add(1), area.width, body_rows),
                cx,
            );
        }
        let message = self.message();
        if !message.is_empty() && rest > body_rows {
            let color = if self.error.is_empty() {
                text(cx, Role::Muted).fg
            } else {
                text(cx, Role::Destructive).fg
            };
            let style = self.dim(page.fg(color));
            let shown = clip_display_prefix(message, area.width);
            cx.put_str(
                area.x,
                area.y.saturating_add(1).saturating_add(body_rows),
                shown,
                style,
            );
        }
    }

    fn measure(&self, avail: Size) -> Size {
        let label = width(self.label) + if self.required { width(MARKER) } else { 0 };
        let body = self.body.map_or(Size::new(0, 0), |b| b.measure(avail));
        let widest = label.max(width(self.message()));
        let w = u16::try_from(widest).unwrap_or(u16::MAX).max(body.width);
        let h = 1_u16
            .saturating_add(self.body_rows(avail))
            .saturating_add(u16::from(!self.message().is_empty()));
        Size::new(w, h).clamp(avail)
    }
}
