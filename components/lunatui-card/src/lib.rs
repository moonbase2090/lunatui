// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::{Attrs, Rect, Size, Style, clip_display_prefix, text_display_width};
use lunatui_ui_tokens::UiToken;
use lunatui_widgets::{Block, BorderType, Padding, RenderCx, Widget};

/// Bordered raised surface with a title, description and body.
///
/// # Example
///
/// ```
/// use lunatui_card::Card;
/// use lunatui_widgets::Widget;
///
/// let card = Card::new().title("Billing").description("Plan and invoices");
/// assert_eq!(card.measure(lunatui_core::Size::new(40, 10)).height, 4);
/// ```
#[derive(Clone, Copy)]
pub struct Card<'a> {
    title: Option<&'a str>,
    description: Option<&'a str>,
    body: Option<&'a dyn Widget>,
    border_type: BorderType,
}

impl Default for Card<'_> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> Card<'a> {
    /// Empty card with a rounded border.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            title: None,
            description: None,
            body: None,
            border_type: BorderType::Rounded,
        }
    }

    /// Bold heading line.
    #[must_use]
    pub const fn title(mut self, title: &'a str) -> Self {
        self.title = Some(title);
        self
    }

    /// Muted line under the title.
    #[must_use]
    pub const fn description(mut self, description: &'a str) -> Self {
        self.description = Some(description);
        self
    }

    /// Widget drawn below the header.
    #[must_use]
    pub const fn body(mut self, body: &'a dyn Widget) -> Self {
        self.body = Some(body);
        self
    }

    /// Border glyph set.
    #[must_use]
    pub const fn border_type(mut self, border_type: BorderType) -> Self {
        self.border_type = border_type;
        self
    }

    fn block(&self, cx: &RenderCx<'_>) -> Block {
        Block::bordered()
            .border_type(self.border_type)
            .border_style(
                Style::new()
                    .fg(UiToken::Border.color(cx))
                    .bg(UiToken::Card.color(cx)),
            )
            .style(
                Style::new()
                    .fg(UiToken::Foreground.color(cx))
                    .bg(UiToken::Card.color(cx)),
            )
            .padding(Padding {
                left: 1,
                right: 1,
                top: 0,
                bottom: 0,
            })
    }

    fn header_rows(&self) -> u16 {
        u16::from(self.title.is_some()) + u16::from(self.description.is_some())
    }
}

impl Widget for Card<'_> {
    fn render(&self, area: Rect, cx: &mut RenderCx<'_>) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let block = self.block(cx);
        block.render(area, cx);
        let inner = block.inner(area);
        if inner.width == 0 || inner.height == 0 {
            return;
        }
        let surface = UiToken::Card.color(cx);
        let mut y = inner.y;
        let bottom = inner.y + inner.height;
        if let Some(title) = self.title {
            let style = Style::new()
                .fg(UiToken::Foreground.color(cx))
                .bg(surface)
                .with_attrs(Attrs::new().set_bold(true));
            cx.put_str(inner.x, y, clip_display_prefix(title, inner.width), style);
            y += 1;
        }
        if let (Some(description), true) = (self.description, y < bottom) {
            let style = Style::new().fg(UiToken::Muted.color(cx)).bg(surface);
            cx.put_str(
                inner.x,
                y,
                clip_display_prefix(description, inner.width),
                style,
            );
            y += 1;
        }
        if let Some(body) = self.body {
            if self.header_rows() > 0 {
                y += 1;
            }
            if y < bottom {
                body.render(Rect::new(inner.x, y, inner.width, bottom - y), cx);
            }
        }
    }

    fn measure(&self, avail: Size) -> Size {
        let chrome_w: u16 = 4;
        let chrome_h: u16 = 2;
        let inner_avail = Size::new(
            avail.width.saturating_sub(chrome_w),
            avail.height.saturating_sub(chrome_h),
        );
        let header_w = [self.title, self.description]
            .into_iter()
            .flatten()
            .map(|text| u16::try_from(text_display_width(text)).unwrap_or(u16::MAX))
            .max()
            .unwrap_or(0);
        let mut width = header_w;
        let mut height = self.header_rows();
        if let Some(body) = self.body {
            let body_size = body.measure(inner_avail);
            width = width.max(body_size.width);
            height = height
                .saturating_add(u16::from(height > 0))
                .saturating_add(body_size.height);
        }
        Size::new(
            width.saturating_add(chrome_w),
            height.saturating_add(chrome_h),
        )
        .clamp(avail)
    }
}
