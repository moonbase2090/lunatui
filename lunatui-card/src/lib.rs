// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::{Attrs, Rect, Size, Style, width};
use lunatui_ui_tokens::{BORDER, Role, Spacing, surface, text};
use lunatui_widgets::{Block, Padding, RenderCx, Widget};

/// Raised, bordered container with a header, a body widget and a footer.
#[derive(Clone, Copy)]
pub struct Card<'a> {
    title: &'a str,
    description: &'a str,
    footer: &'a str,
    body: Option<&'a dyn Widget>,
}

impl<'a> Card<'a> {
    /// Card with `title` and nothing else.
    #[must_use]
    pub const fn new(title: &'a str) -> Self {
        Self {
            title,
            description: "",
            footer: "",
            body: None,
        }
    }

    /// Muted line under the title.
    #[must_use]
    pub const fn description(mut self, description: &'a str) -> Self {
        self.description = description;
        self
    }

    /// Muted line on the bottom row.
    #[must_use]
    pub const fn footer(mut self, footer: &'a str) -> Self {
        self.footer = footer;
        self
    }

    /// Widget drawn between the header and the footer.
    #[must_use]
    pub const fn body(mut self, body: &'a dyn Widget) -> Self {
        self.body = Some(body);
        self
    }

    fn block(cx: &RenderCx<'_>) -> Block {
        Block::bordered()
            .border_type(BORDER)
            .border_style(surface(cx, Role::Card).fg(text(cx, Role::Border).fg))
            .style(surface(cx, Role::Card))
            .padding(Padding {
                left: Spacing::S2.y(),
                right: Spacing::S2.y(),
                top: Spacing::S1.y(),
                bottom: Spacing::S1.y(),
            })
    }

    fn header_rows(&self) -> u16 {
        1 + u16::from(!self.description.is_empty())
    }
}

impl Widget for Card<'_> {
    fn render(&self, area: Rect, cx: &mut RenderCx<'_>) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let block = Self::block(cx);
        let inner = block.inner(area);
        block.render(area, cx);
        if inner.width == 0 || inner.height == 0 {
            return;
        }
        let title = surface(cx, Role::Card).with_attrs(Attrs::new().set_bold(true));
        put_clipped(cx, inner.x, inner.y, inner.width, self.title, title);
        let mut top = inner.y + 1;
        if !self.description.is_empty() && inner.height > 1 {
            let muted = surface(cx, Role::Card).fg(text(cx, Role::Muted).fg);
            put_clipped(cx, inner.x, top, inner.width, self.description, muted);
            top += 1;
        }
        let mut bottom = inner.y + inner.height;
        if !self.footer.is_empty() && bottom > top + 1 {
            bottom -= 1;
            let muted = surface(cx, Role::Card).fg(text(cx, Role::Muted).fg);
            put_clipped(cx, inner.x, bottom, inner.width, self.footer, muted);
        }
        if let Some(body) = self.body {
            // One blank row between the header and the body.
            let body_top = top + 1;
            if bottom > body_top {
                body.render(
                    Rect::new(inner.x, body_top, inner.width, bottom - body_top),
                    cx,
                );
            }
        }
    }

    fn measure(&self, avail: Size) -> Size {
        let text_w = [self.title, self.description, self.footer]
            .iter()
            .map(|s| width(s))
            .max()
            .unwrap_or(0);
        let body = self.body.map_or(Size::new(0, 0), |b| b.measure(avail));
        let w = u16::try_from(text_w).unwrap_or(u16::MAX).max(body.width) + 6;
        let h = 4
            + self.header_rows()
            + u16::from(self.body.is_some()) * (1 + body.height)
            + u16::from(!self.footer.is_empty());
        Size::new(w, h).clamp(avail)
    }
}

/// Orientation of a [`Separator`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Orientation {
    /// A rule across the area's first row.
    #[default]
    Horizontal,
    /// A rule down the area's first column.
    Vertical,
}

/// Single-cell-thick rule in the border color.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Separator {
    orientation: Orientation,
    on: Role,
}

impl Separator {
    /// Horizontal separator.
    #[must_use]
    pub const fn horizontal() -> Self {
        Self {
            orientation: Orientation::Horizontal,
            on: Role::Background,
        }
    }

    /// Vertical separator.
    #[must_use]
    pub const fn vertical() -> Self {
        Self {
            orientation: Orientation::Vertical,
            on: Role::Background,
        }
    }

    /// Surface the rule sits on, so its cells keep that background.
    #[must_use]
    pub const fn on(mut self, role: Role) -> Self {
        self.on = role;
        self
    }
}

impl Widget for Separator {
    fn render(&self, area: Rect, cx: &mut RenderCx<'_>) {
        let style = surface(cx, self.on).fg(text(cx, Role::Border).fg);
        match self.orientation {
            Orientation::Horizontal if area.height > 0 => {
                cx.fill(Rect::new(area.x, area.y, area.width, 1), style, '─');
            }
            Orientation::Vertical if area.width > 0 => {
                cx.fill(Rect::new(area.x, area.y, 1, area.height), style, '│');
            }
            _ => {}
        }
    }

    fn measure(&self, avail: Size) -> Size {
        match self.orientation {
            Orientation::Horizontal => Size::new(avail.width, 1),
            Orientation::Vertical => Size::new(1, avail.height),
        }
    }
}

/// Centered title and description for a blank state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Empty<'a> {
    title: &'a str,
    description: &'a str,
    on: Role,
}

impl<'a> Empty<'a> {
    /// Blank state with `title` and `description`.
    #[must_use]
    pub const fn new(title: &'a str, description: &'a str) -> Self {
        Self {
            title,
            description,
            on: Role::Background,
        }
    }

    /// Surface the text sits on, so its cells keep that background.
    #[must_use]
    pub const fn on(mut self, role: Role) -> Self {
        self.on = role;
        self
    }
}

impl Widget for Empty<'_> {
    fn render(&self, area: Rect, cx: &mut RenderCx<'_>) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let rows = 1 + u16::from(!self.description.is_empty());
        let top = area.y + area.height.saturating_sub(rows) / 2;
        let base = surface(cx, self.on);
        let bold = base.with_attrs(Attrs::new().set_bold(true));
        centered(cx, area, top, self.title, bold);
        if !self.description.is_empty() && top + 1 < area.y + area.height {
            let muted = base.fg(text(cx, Role::Muted).fg);
            centered(cx, area, top + 1, self.description, muted);
        }
    }

    fn measure(&self, avail: Size) -> Size {
        let w = width(self.title).max(width(self.description));
        Size::new(
            u16::try_from(w).unwrap_or(u16::MAX),
            1 + u16::from(!self.description.is_empty()),
        )
        .clamp(avail)
    }
}

fn centered(cx: &mut RenderCx<'_>, area: Rect, y: u16, s: &str, style: Style) {
    let w = u16::try_from(width(s)).unwrap_or(u16::MAX).min(area.width);
    let x = area.x + (area.width - w) / 2;
    put_clipped(cx, x, y, w, s, style);
}

fn put_clipped(cx: &mut RenderCx<'_>, x: u16, y: u16, max: u16, s: &str, style: Style) {
    let clipped = lunatui_core::clip_display_prefix(s, max);
    cx.put_str(x, y, clipped, style);
}
