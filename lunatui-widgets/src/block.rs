// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_core::{Rect, Size, Style, symbols::border, width};

use crate::borders::{BorderType, Borders};
use crate::padding::Padding;
use crate::{RenderCx, Widget};

/// Horizontal alignment for block titles.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Alignment {
    /// Flush left in the title band.
    #[default]
    Left,
    /// Centered in the title band.
    Center,
    /// Flush right in the title band.
    Right,
}

/// Whether a title sits on the top or bottom edge.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TitlePosition {
    /// Top edge (default for [`Block::title`]).
    #[default]
    Top,
    /// Bottom edge.
    Bottom,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct TitleEntry {
    position: Option<TitlePosition>,
    text: String,
    alignment: Option<Alignment>,
}

/// Bordered container with optional titles (ratatui `Block` parity).
#[derive(Clone, Debug)]
pub struct Block {
    titles: Vec<TitleEntry>,
    titles_style: Style,
    titles_alignment: Alignment,
    titles_position: TitlePosition,
    borders: Borders,
    border_style: Style,
    border_set: border::Set<'static>,
    style: Style,
    padding: Padding,
}

impl Default for Block {
    fn default() -> Self {
        Self::new()
    }
}

impl Block {
    /// Block with no borders.
    #[must_use]
    pub fn new() -> Self {
        Self {
            titles: Vec::new(),
            titles_style: Style::new(),
            titles_alignment: Alignment::Left,
            titles_position: TitlePosition::Top,
            borders: Borders::NONE,
            border_style: Style::new(),
            border_set: BorderType::Plain.border_set(),
            style: Style::new(),
            padding: Padding::ZERO,
        }
    }

    /// Block with all borders shown.
    #[must_use]
    pub fn bordered() -> Self {
        Self::new().borders(Borders::ALL)
    }

    /// Visible borders.
    #[must_use]
    pub fn borders(mut self, borders: Borders) -> Self {
        self.borders = borders;
        self
    }

    /// Border glyph style.
    #[must_use]
    pub fn border_style(mut self, style: Style) -> Self {
        self.border_style = style;
        self
    }

    /// Border symbol set from `border_type`.
    #[must_use]
    pub fn border_type(mut self, border_type: BorderType) -> Self {
        self.border_set = border_type.border_set();
        self
    }

    /// Background style for the block area.
    #[must_use]
    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    /// Inner padding.
    #[must_use]
    pub fn padding(mut self, padding: Padding) -> Self {
        self.padding = padding;
        self
    }

    /// Default title alignment for titles without their own.
    #[must_use]
    pub fn title_alignment(mut self, alignment: Alignment) -> Self {
        self.titles_alignment = alignment;
        self
    }

    /// Default title position for [`Block::title`].
    #[must_use]
    pub fn title_position(mut self, position: TitlePosition) -> Self {
        self.titles_position = position;
        self
    }

    /// Style applied to every title line.
    #[must_use]
    pub fn title_style(mut self, style: Style) -> Self {
        self.titles_style = style;
        self
    }

    /// Adds a title using the default position.
    #[must_use]
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.titles.push(TitleEntry {
            position: None,
            text: title.into(),
            alignment: None,
        });
        self
    }

    /// Adds a top title with explicit alignment.
    #[must_use]
    pub fn title_top_aligned(mut self, title: impl Into<String>, alignment: Alignment) -> Self {
        self.titles.push(TitleEntry {
            position: Some(TitlePosition::Top),
            text: title.into(),
            alignment: Some(alignment),
        });
        self
    }

    /// Content rectangle inside borders, titles, and padding.
    #[must_use]
    pub fn inner(&self, area: Rect) -> Rect {
        let mut inner = area;
        if self.borders.contains(Borders::LEFT) {
            inner.x = inner.x.saturating_add(1).min(inner.x + inner.width);
            inner.width = inner.width.saturating_sub(1);
        }
        if self.borders.contains(Borders::TOP) || self.has_title_at(TitlePosition::Top) {
            inner.y = inner.y.saturating_add(1).min(inner.y + inner.height);
            inner.height = inner.height.saturating_sub(1);
        }
        if self.borders.contains(Borders::RIGHT) {
            inner.width = inner.width.saturating_sub(1);
        }
        if self.borders.contains(Borders::BOTTOM) || self.has_title_at(TitlePosition::Bottom) {
            inner.height = inner.height.saturating_sub(1);
        }
        inner.x = inner.x.saturating_add(self.padding.left);
        inner.y = inner.y.saturating_add(self.padding.top);
        let horizontal = self.padding.left.saturating_add(self.padding.right);
        let vertical = self.padding.top.saturating_add(self.padding.bottom);
        inner.width = inner.width.saturating_sub(horizontal);
        inner.height = inner.height.saturating_sub(vertical);
        inner
    }

    fn has_title_at(&self, position: TitlePosition) -> bool {
        self.titles
            .iter()
            .any(|entry| entry.position.unwrap_or(self.titles_position) == position)
    }

    fn render_borders(&self, area: Rect, cx: &mut RenderCx<'_>) {
        let left = area.x;
        let top = area.y;
        let right = area.x.saturating_add(area.width.saturating_sub(1));
        let bottom = area.y.saturating_add(area.height.saturating_sub(1));

        if self.borders.contains(Borders::LEFT) {
            for y in top..=bottom {
                put_symbol(
                    cx,
                    left,
                    y,
                    self.border_set.vertical_left,
                    self.border_style,
                );
            }
        }
        if self.borders.contains(Borders::TOP) {
            for x in left..=right {
                put_symbol(
                    cx,
                    x,
                    top,
                    self.border_set.horizontal_top,
                    self.border_style,
                );
            }
        }
        if self.borders.contains(Borders::RIGHT) {
            for y in top..=bottom {
                put_symbol(
                    cx,
                    right,
                    y,
                    self.border_set.vertical_right,
                    self.border_style,
                );
            }
        }
        if self.borders.contains(Borders::BOTTOM) {
            for x in left..=right {
                put_symbol(
                    cx,
                    x,
                    bottom,
                    self.border_set.horizontal_bottom,
                    self.border_style,
                );
            }
        }

        let corners = [
            (
                Borders::LEFT | Borders::TOP,
                left,
                top,
                self.border_set.top_left,
            ),
            (
                Borders::RIGHT | Borders::TOP,
                right,
                top,
                self.border_set.top_right,
            ),
            (
                Borders::LEFT | Borders::BOTTOM,
                left,
                bottom,
                self.border_set.bottom_left,
            ),
            (
                Borders::RIGHT | Borders::BOTTOM,
                right,
                bottom,
                self.border_set.bottom_right,
            ),
        ];
        for (mask, x, y, sym) in corners {
            if self.borders.contains_all(mask) {
                put_symbol(cx, x, y, sym, self.border_style);
            }
        }
    }

    fn render_titles(&self, area: Rect, cx: &mut RenderCx<'_>) {
        self.render_titles_at(TitlePosition::Top, area, cx);
        self.render_titles_at(TitlePosition::Bottom, area, cx);
    }

    fn render_titles_at(&self, position: TitlePosition, area: Rect, cx: &mut RenderCx<'_>) {
        self.render_aligned_titles(position, Alignment::Left, area, cx);
        self.render_aligned_titles(position, Alignment::Center, area, cx);
        self.render_aligned_titles(position, Alignment::Right, area, cx);
    }

    fn render_aligned_titles(
        &self,
        position: TitlePosition,
        alignment: Alignment,
        area: Rect,
        cx: &mut RenderCx<'_>,
    ) {
        let band = self.titles_band(area, position);
        if band.width == 0 || band.height == 0 {
            return;
        }
        let titles: Vec<&TitleEntry> = self
            .titles
            .iter()
            .filter(|entry| entry.position.unwrap_or(self.titles_position) == position)
            .filter(|entry| entry.alignment.unwrap_or(self.titles_alignment) == alignment)
            .collect();
        if titles.is_empty() {
            return;
        }
        let style = self.titles_style;
        match alignment {
            Alignment::Left => {
                let mut x = band.x;
                for (index, entry) in titles.iter().enumerate() {
                    let w = title_width(entry);
                    if x >= band.x + band.width {
                        break;
                    }
                    put_symbol(cx, x, band.y, &entry.text, style);
                    x = x.saturating_add(w);
                    if index + 1 < titles.len() {
                        x = x.saturating_add(1);
                    }
                }
            }
            Alignment::Center => {
                let total = titles
                    .iter()
                    .map(|entry| title_width(entry).saturating_add(1))
                    .fold(0_u16, u16::saturating_add)
                    .saturating_sub(1);
                let x = band.x.saturating_add(band.width.saturating_sub(total) / 2);
                let mut cursor = x;
                for (index, entry) in titles.iter().enumerate() {
                    put_symbol(cx, cursor, band.y, &entry.text, style);
                    cursor = cursor.saturating_add(title_width(entry));
                    if index + 1 < titles.len() {
                        cursor = cursor.saturating_add(1);
                    }
                }
            }
            Alignment::Right => {
                let total = titles
                    .iter()
                    .map(|entry| title_width(entry).saturating_add(1))
                    .fold(0_u16, u16::saturating_add)
                    .saturating_sub(1);
                let mut x = band.x.saturating_add(band.width.saturating_sub(total));
                for (index, entry) in titles.iter().enumerate() {
                    put_symbol(cx, x, band.y, &entry.text, style);
                    x = x.saturating_add(title_width(entry));
                    if index + 1 < titles.len() {
                        x = x.saturating_add(1);
                    }
                }
            }
        }
    }

    fn titles_band(&self, area: Rect, position: TitlePosition) -> Rect {
        let left_border = u16::from(self.borders.contains(Borders::LEFT));
        let right_border = u16::from(self.borders.contains(Borders::RIGHT));
        let y = match position {
            TitlePosition::Top => area.y,
            TitlePosition::Bottom => area.y.saturating_add(area.height.saturating_sub(1)),
        };
        Rect {
            x: area.x.saturating_add(left_border),
            y,
            width: area
                .width
                .saturating_sub(left_border)
                .saturating_sub(right_border),
            height: 1,
        }
    }
}

impl Widget for Block {
    fn render(&self, area: Rect, cx: &mut RenderCx<'_>) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        cx.set_style(area, self.style);
        self.render_borders(area, cx);
        self.render_titles(area, cx);
    }

    fn measure(&self, avail: Size) -> Size {
        Size::new(2, 2).clamp(avail)
    }
}

fn title_width(entry: &TitleEntry) -> u16 {
    u16::try_from(width(&entry.text)).unwrap_or(u16::MAX)
}

fn put_symbol(cx: &mut RenderCx<'_>, x: u16, y: u16, text: &str, style: Style) {
    if !text.is_empty() {
        cx.put_str(x, y, text, style);
    }
}
